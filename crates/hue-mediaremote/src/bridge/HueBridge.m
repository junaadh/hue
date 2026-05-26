#import "HueBridge.h"
#import <Foundation/Foundation.h>
#import <dlfcn.h>

typedef void (*MRSendCommandFn)(unsigned int command, id options);
static void *MR = NULL;
static MRSendCommandFn MRSendCommand = NULL;

static HueEventCallback HUE_EVENT_CALLBACK = NULL;
static NSString *LAST_PERSISTENT_ID = nil;
static NSString *LAST_PLAYER_STATE = nil;

#pragma mark - Small Helpers

/// Duplicate an NSString into malloc-owned UTF-8 memory.
///
/// Rust owns the copied C string after receiving it through FFI,
/// and must eventually free it through the matching Hue free function.
static char *dup_nsstring(NSString *s) {
    if (!s) s = @"";
    const char *utf8 = [s UTF8String];
    return strdup(utf8 ? utf8 : "");
}

/// Escape text for embedding inside a simple AppleScript string literal.
static NSString *escape_applescript_string(NSString *s) {
    NSString *escaped = [s stringByReplacingOccurrencesOfString:@"\\"
                                                     withString:@"\\\\"];
    escaped = [escaped stringByReplacingOccurrencesOfString:@"\""
                                                 withString:@"\\\""];
    return escaped;
}

/// Convert a song title into a safe filename.
static NSString *sanitize_filename(NSString *name) {
    if (!name || [name length] == 0) {
        return @"current-artwork";
    }
    NSCharacterSet *bad =
        [NSCharacterSet characterSetWithCharactersInString:@"/\\?%*|\"<>:"];
    NSArray<NSString *> *parts =
        [name componentsSeparatedByCharactersInSet:bad];
    NSString *joined = [parts componentsJoinedByString:@"_"];
    if ([joined length] == 0) {
        return @"current-artwork";
    }
    return joined;
}

/// Return Hue's app cache directory:
///
/// ~/Library/Caches/hua-mediaremote/artwork
static NSString *hue_artwork_cache_dir(void) {
    NSArray<NSString *> *dirs =
        NSSearchPathForDirectoriesInDomains(NSCachesDirectory,
                                            NSUserDomainMask,
                                            YES);
    NSString *base = [dirs firstObject];
    if (!base) {
        base = NSTemporaryDirectory();
    }
    NSString *dir =
        [[base stringByAppendingPathComponent:@"hua-mediaremote"]
              stringByAppendingPathComponent:@"artwork"];
    [[NSFileManager defaultManager] createDirectoryAtPath:dir
                              withIntermediateDirectories:YES
                                               attributes:nil
                                                    error:nil];
    return dir;
}

/// Return the expected artwork path for a song title.
///
/// Example:
/// ~/Library/Caches/hua-mediaremote/artwork/Crazy Train.jpg
static NSString *hue_artwork_path_for_song(NSString *songName) {
    NSString *safe = sanitize_filename(songName);
    NSString *file = [safe stringByAppendingPathExtension:@"jpg"];
    return [hue_artwork_cache_dir() stringByAppendingPathComponent:file];
}
/// Run AppleScript and return its string result.
static NSString *run_script(NSString *source) {
    NSAppleScript *script = [[NSAppleScript alloc] initWithSource:source];
    NSDictionary *error = nil;
    NSAppleEventDescriptor *result =
        [script executeAndReturnError:&error];
    if (!result) {
        NSLog(@"Hue AppleScript error: %@", error);
        return nil;
    }
    return [result stringValue];
}

#pragma mark - Artwork

/// Export current Music.app artwork into Hue's cache directory.
///
/// The artwork is written as:
///
/// ~/Library/Caches/hua-mediaremote/artwork/<song_name>.jpg
///
/// Returns a retained NSString path owned by Objective-C autorelease pool.
/// The caller duplicates it if it crosses the FFI boundary.
static NSString *run_artwork_script_for_song(NSString *songName) {
    NSString *path = hue_artwork_path_for_song(songName);
    NSString *escapedPath = escape_applescript_string(path);
    NSString *fileScript = [NSString stringWithFormat:
        @"tell application \"Music\"\n"
        @"    set artData to data of artwork 1 of current track\n"
        @"end tell\n"
        @"set outPath to \"%@\"\n"
        @"set fileRef to open for access POSIX file outPath with write permission\n"
        @"set eof fileRef to 0\n"
        @"write artData to fileRef\n"
        @"close access fileRef\n"
        @"return outPath\n",
        escapedPath
    ];
    return run_script(fileScript);
}

#pragma mark - MediaRemote Commands

/// Lazily load MediaRemote and resolve command sending.
///
/// Metadata is intentionally not read through MediaRemote because recent macOS
/// versions return nil for now-playing dictionaries in normal processes.
/// Commands still work, so Hue uses MediaRemote for play/pause/next/previous.
static int mr_init(void) {
    if (MR) return 0;
    MR = dlopen("/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote",
                RTLD_LAZY);
    if (!MR) return -1;
    MRSendCommand =
        (MRSendCommandFn)dlsym(MR, "MRMediaRemoteSendCommand");
    if (!MRSendCommand) return -2;
    return 0;
}

#pragma mark - Playback Fetching

/// Fetch a full playback snapshot from Music.app.
///
/// This is used on startup and on track changes. It includes:
/// - title
/// - artist
/// - album
/// - app name
/// - position
/// - duration
/// - playing flag
/// - cached artwork file path
int hue_get_playback_info(HuePlaybackInfo *out) {
    if (!out) return -10;
    memset(out, 0, sizeof(HuePlaybackInfo));
    NSString *metadataScript =
        @"tell application \"Music\"\n"
        @"    set s to player state as string\n"
        @"    if s is \"stopped\" then\n"
        @"        return \"stopped|||"
        @"|||"
        @"|||"
        @"|||0|||0\"\n"
        @"    end if\n"
        @"    set t to name of current track\n"
        @"    set ar to artist of current track\n"
        @"    set al to album of current track\n"
        @"    set p to player position\n"
        @"    set d to duration of current track\n"
        @"    return s & \"|||\" & t & \"|||\" & ar & \"|||\" & al & \"|||\" & (p as string) & \"|||\" & (d as string)\n"
        @"end tell\n";
    NSString *raw = run_script(metadataScript);
    if (!raw) return -20;
    NSArray<NSString *> *parts =
        [raw componentsSeparatedByString:@"|||"];
    if ([parts count] < 6) return -21;
    NSString *state = parts[0];
    NSString *title = parts[1];
    NSString *artist = parts[2];
    NSString *album = parts[3];
    NSString *position = parts[4];
    NSString *duration = parts[5];
    out->song_info.title = dup_nsstring(title);
    out->song_info.artist = dup_nsstring(artist);
    out->song_info.album = dup_nsstring(album);
    out->song_info.app_name = dup_nsstring(@"Music");
    out->progress_info.position = [position doubleValue];
    out->progress_info.duration = [duration doubleValue];
    out->playing = [state isEqualToString:@"playing"] ? 1 : 0;
    NSString *artPath = run_artwork_script_for_song(title);
    out->artwork_info.path = dup_nsstring(artPath);
    return 0;
}

/// Free a HuePlaybackInfo returned by hue_get_playback_info.
void hue_free_playback_info(HuePlaybackInfo *info) {
    if (!info) return;
    if (info->song_info.title) {
        free((void *)info->song_info.title);
    }
    if (info->song_info.artist) {
        free((void *)info->song_info.artist);
    }
    if (info->song_info.album) {
        free((void *)info->song_info.album);
    }
    if (info->song_info.app_name) {
        free((void *)info->song_info.app_name);
    }
    if (info->artwork_info.path) {
        free((void *)info->artwork_info.path);
    }
    memset(info, 0, sizeof(HuePlaybackInfo));
}

/// Fetch only progress data.
///
/// This should be used for cheap progress-bar updates.
int hue_get_progress(HueProgressInfo *out) {
    if (!out) return -10;
    memset(out, 0, sizeof(HueProgressInfo));
    NSString *script =
        @"tell application \"Music\"\n"
        @"    if player state is stopped then return \"0|||0\"\n"
        @"    set p to player position\n"
        @"    set d to duration of current track\n"
        @"    return (p as string) & \"|||\" & (d as string)\n"
        @"end tell\n";
    NSString *raw = run_script(script);
    if (!raw) return -20;
    NSArray<NSString *> *parts =
        [raw componentsSeparatedByString:@"|||"];
    if ([parts count] < 2) return -21;
    out->position = [parts[0] doubleValue];
    out->duration = [parts[1] doubleValue];
    return 0;
}

/// Export and return artwork path only.
///
/// Used when Rust determines the track changed and wants artwork refresh
/// without refetching all metadata.
int hue_get_artwork(HueArtworkInfo *out) {
    if (!out) return -10;
    memset(out, 0, sizeof(HueArtworkInfo));
    NSString *titleScript =
        @"tell application \"Music\"\n"
        @"    if player state is stopped then return \"current-artwork\"\n"
        @"    return name of current track\n"
        @"end tell\n";
    NSString *title = run_script(titleScript);
    if (!title) return -20;
    NSString *path = run_artwork_script_for_song(title);
    if (!path) return -21;
    out->path = dup_nsstring(path);
    return 0;
}

/// Free artwork path returned by hue_get_artwork.
void hue_free_artwork(HueArtworkInfo *out) {
    if (!out) return;
    if (out->path) {
        free((void *)out->path);
    }
    memset(out, 0, sizeof(HueArtworkInfo));
}
#pragma mark - Commands
int hue_play_pause(void) {
    int init = mr_init();
    if (init != 0) return init;
    MRSendCommand(2, nil);
    return 0;
}
int hue_next_track(void) {
    int init = mr_init();
    if (init != 0) return init;
    MRSendCommand(4, nil);
    return 0;
}
int hue_previous_track(void) {
    int init = mr_init();
    if (init != 0) return init;
    MRSendCommand(5, nil);
    return 0;
}

#pragma mark - Events

/// Emit a stack-owned event.
///
/// The callback must copy any data it wants to keep. The event pointer is only
/// valid for the duration of the callback.
static void hue_emit_event(HueEvent *event) {
    if (HUE_EVENT_CALLBACK) {
        HUE_EVENT_CALLBACK(event);
    }
}

/// Convert Music.app notification userInfo into a Hue event kind.
///
/// Rules:
/// - first notification initializes cache and is treated as metadata
/// - PersistentID change means track changed
/// - Player State change means playback changed
/// - otherwise metadata changed
static HueEventKind hue_event_kind_from_music_user_info(NSDictionary *info) {
    NSString *persistentId =
        [[info objectForKey:@"PersistentID"] description];
    NSString *playerState =
        [[info objectForKey:@"Player State"] description];
    if (!LAST_PERSISTENT_ID || !LAST_PLAYER_STATE) {
        if (persistentId) LAST_PERSISTENT_ID = [persistentId copy];
        if (playerState) LAST_PLAYER_STATE = [playerState copy];
        return HUE_EVENT_METADATA;
    }
    BOOL trackChanged =
        persistentId &&
        ![persistentId isEqualToString:LAST_PERSISTENT_ID];
    BOOL playbackChanged =
        playerState &&
        ![playerState isEqualToString:LAST_PLAYER_STATE];
    if (persistentId) {
        LAST_PERSISTENT_ID = [persistentId copy];
    }
    if (playerState) {
        LAST_PLAYER_STATE = [playerState copy];
    }
    if (trackChanged) {
        return HUE_EVENT_TRACK_CHANGED;
    }
    if (playbackChanged) {
        return HUE_EVENT_PLAYBACK;
    }
    return HUE_EVENT_METADATA;
}

/// Fill event data from Music.app notification userInfo.
///
/// Notification userInfo usually contains:
/// - Name
/// - Artist
/// - Album
/// - Player State
/// - Total Time in milliseconds
/// - PersistentID
///
/// It does not contain current position or artwork bytes.
/// Rust should call hue_get_progress for accurate position, and hue_get_artwork
/// only if the track changed.
static void hue_fill_event_from_music_user_info(HueEvent *event,
                                                NSDictionary *info) {
    HueEventKind kind = hue_event_kind_from_music_user_info(info);
    memset(event, 0, sizeof(HueEvent));
    event->kind = kind;
    NSString *playerState =
        [[info objectForKey:@"Player State"] description];
    int playing = [playerState isEqualToString:@"Playing"] ? 1 : 0;
    if (kind == HUE_EVENT_PLAYBACK) {
        event->data.playback.playing = playing;
        return;
    }
    if (kind == HUE_EVENT_TRACK_CHANGED ||
        kind == HUE_EVENT_METADATA) {
        NSString *title =
            [[info objectForKey:@"Name"] description];
        NSString *artist =
            [[info objectForKey:@"Artist"] description];
        NSString *album =
            [[info objectForKey:@"Album"] description];
        NSNumber *totalTime =
            [info objectForKey:@"Total Time"];
        HuePlaybackInfo *p = NULL;
        if (kind == HUE_EVENT_TRACK_CHANGED) {
            p = &event->data.changed.playback_info;
        } else {
            p = &event->data.metadata.playback_info;
        }
        p->song_info.title = [title UTF8String];
        p->song_info.artist = [artist UTF8String];
        p->song_info.album = [album UTF8String];
        p->song_info.app_name = "Music";

        HueProgressInfo progress;
        memset(&progress, 0, sizeof(HueProgressInfo));

        if (hue_get_progress(&progress) == 0) {
            p->progress_info = progress;
        } else {
            p->progress_info.position = 0.0;
            p->progress_info.duration =
                totalTime ? ([totalTime doubleValue] / 1000.0) : 0.0;
        }

        p->playing = playing;

        NSString *artPath = run_artwork_script_for_song(title);
        p->artwork_info.path = [artPath UTF8String];
    }
}

/// Register media notifications.
///
/// `com.apple.Music.playerInfo` is the useful one.
/// MediaRemote notifications are kept only as experimental wake signals.
static void hue_register_media_notifications(void) {
    NSDistributedNotificationCenter *center =
        [NSDistributedNotificationCenter defaultCenter];
    [center addObserverForName:@"com.apple.Music.playerInfo"
                        object:nil
                         queue:[NSOperationQueue mainQueue]
                    usingBlock:^(NSNotification *note) {
        NSDictionary *info = note.userInfo;
        HueEvent event;
        hue_fill_event_from_music_user_info(&event, info);
        hue_emit_event(&event);
    }];
    NSArray<NSString *> *mrNames = @[
        @"kMRMediaRemoteNowPlayingInfoDidChangeNotification",
        @"kMRMediaRemoteNowPlayingApplicationPlaybackStateDidChangeNotification",
    ];
    for (NSString *name in mrNames) {
        [center addObserverForName:name
                            object:nil
                             queue:[NSOperationQueue mainQueue]
                        usingBlock:^(NSNotification *note) {
            HueEvent event;
            memset(&event, 0, sizeof(HueEvent));
            event.kind = HUE_EVENT_METADATA;
            hue_emit_event(&event);
        }];
    }
}

/// Register Rust callback for Hue events.
int hue_register_notifications(HueEventCallback cb) {
    HUE_EVENT_CALLBACK = cb;
    hue_register_media_notifications();
    return 0;
}

/// Run macOS main run loop.
///
/// Required for NSDistributedNotificationCenter callbacks to fire.
void hue_run_loop(void) {
    [[NSRunLoop mainRunLoop] run];
}
