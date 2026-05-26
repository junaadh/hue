#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct HueSongInfo {
  const char *title;
  const char *artist;
  const char *album;
  const char *app_name;
} HueSongInfo;

typedef struct HueProgressInfo {
  double position;
  double duration;
} HueProgressInfo;

typedef struct HueArtworkInfo {
  const char *path;
} HueArtworkInfo;

typedef struct HuePlaybackInfo {
  HueSongInfo song_info;
  HueProgressInfo progress_info;

  int playing;

  HueArtworkInfo artwork_info;
} HuePlaybackInfo;

typedef enum HueEventKind {
  HUE_EVENT_PLAYBACK = 0,
  HUE_EVENT_TRACK_CHANGED = 1,
  HUE_EVENT_METADATA = 2,
} HueEventKind;

typedef struct HueEvent {
  HueEventKind kind;
  union {
    struct {
      int playing;
    } playback;
    
    struct {
      HuePlaybackInfo playback_info;
    } changed;

    struct {
      HuePlaybackInfo playback_info;
    } metadata;
  } data;
} HueEvent;

typedef void (*HueEventCallback)(const HueEvent *event);

int hue_get_playback_info(HuePlaybackInfo *out);
void hue_free_playback_info(HuePlaybackInfo *info);

int hue_get_progress(HueProgressInfo *out);

int hue_get_artwork(HueArtworkInfo *out);
void hue_free_artwork(HueArtworkInfo *out);

int hue_register_notifications(HueEventCallback cb);
void hue_run_loop(void);

int hue_play_pause(void);
int hue_next_track(void);
int hue_previous_track(void);

#ifdef __cplusplus
}
#endif
