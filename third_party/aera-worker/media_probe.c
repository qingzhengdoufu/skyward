// SPDX-License-Identifier: Apache-2.0
// Offline ARM64 GStreamer test. Does not open GPU, soundcards or partitions.
#include <gst/gst.h>
#include <stdio.h>

static guint frames;
static guint changed;
static guint64 previous;
static gint audio_buffers;
static gint nonzero_audio;

static GstPadProbeReturn pcm(GstPad *pad, GstPadProbeInfo *info, gpointer data) {
  (void)pad; (void)data;
  GstBuffer *buffer = GST_PAD_PROBE_INFO_BUFFER(info);
  GstMapInfo map;
  if (!buffer || !gst_buffer_map(buffer, &map, GST_MAP_READ)) return GST_PAD_PROBE_OK;
  g_atomic_int_inc(&audio_buffers);
  for (gsize i = 0; i < map.size; ++i) {
    if (map.data[i]) { g_atomic_int_inc(&nonzero_audio); break; }
  }
  gst_buffer_unmap(buffer, &map);
  return GST_PAD_PROBE_OK;
}

static void frame(GstElement *sink, GstBuffer *buffer, GstPad *pad, gpointer data) {
  (void)sink; (void)pad; (void)data;
  GstMapInfo map;
  if (!gst_buffer_map(buffer, &map, GST_MAP_READ)) return;
  guint64 checksum = 1469598103934665603ULL;
  for (gsize i = 0; i < map.size; i += 97) checksum = (checksum ^ map.data[i]) * 1099511628211ULL;
  if (frames && checksum != previous) ++changed;
  previous = checksum;
  ++frames;
  gst_buffer_unmap(buffer, &map);
}

static gboolean errors(GstBus *bus) {
  GstMessage *message = gst_bus_pop_filtered(bus, GST_MESSAGE_ERROR);
  if (!message) return FALSE;
  GError *error = NULL; gchar *detail = NULL;
  gst_message_parse_error(message, &error, &detail);
  fprintf(stderr, "MEDIA_ERROR: %s (%s)\n", error ? error->message : "unknown", detail ? detail : "");
  g_clear_error(&error); g_free(detail); gst_message_unref(message);
  return TRUE;
}

static gboolean wait_state(GstElement *player, GstState state) {
  return gst_element_get_state(player, NULL, NULL, 10 * GST_SECOND) != GST_STATE_CHANGE_FAILURE
      && GST_STATE(player) == state;
}

int main(int argc, char **argv) {
  gst_init(&argc, &argv);
  if (argc != 2) return 64;
  const char *required[] = {"autoaudiosink", "scaletempo", "webvttenc", "qtdemux", "openh264dec", "fdkaacdec", NULL};
  for (unsigned i = 0; required[i]; ++i) {
    GstElementFactory *factory = gst_element_factory_find(required[i]);
    if (!factory) { fprintf(stderr, "MISSING: %s\n", required[i]); return 2; }
    printf("ELEMENT_OK: %s\n", required[i]); gst_object_unref(factory);
  }
  GstElement *player = gst_element_factory_make("playbin", NULL);
  GstElement *video = gst_element_factory_make("fakesink", NULL);
  GstElement *audio = gst_element_factory_make("autoaudiosink", NULL);
  if (!player || !video || !audio || !GST_IS_CHILD_PROXY(audio)) return 3;
  GstPad *audio_pad = gst_element_get_static_pad(audio, "sink");
  if (!audio_pad) return 15;
  gst_pad_add_probe(audio_pad, GST_PAD_PROBE_TYPE_BUFFER, pcm, NULL, NULL);
  gst_object_unref(audio_pad);
  g_object_set(video, "sync", TRUE, "signal-handoffs", TRUE, NULL);
  g_signal_connect(video, "handoff", G_CALLBACK(frame), NULL);
  g_object_set(player, "uri", argv[1], "video-sink", video, "audio-sink", audio, NULL);
  GstBus *bus = gst_element_get_bus(player);
  if (gst_element_set_state(player, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE || !wait_state(player, GST_STATE_PLAYING)) { errors(bus); return 4; }
  g_usleep(900000);
  if (errors(bus)) return 5;
  gint64 playing = 0, paused = 0, still = 0, sought = 0;
  gst_element_query_position(player, GST_FORMAT_TIME, &playing);
  if (playing < 300 * GST_MSECOND) return 6;
  gst_element_set_state(player, GST_STATE_PAUSED);
  if (!wait_state(player, GST_STATE_PAUSED)) return 7;
  gst_element_query_position(player, GST_FORMAT_TIME, &paused);
  g_usleep(300000);
  gst_element_query_position(player, GST_FORMAT_TIME, &still);
  if (llabs(still - paused) > 30 * GST_MSECOND) return 8;
  if (!gst_element_seek_simple(player, GST_FORMAT_TIME, GST_SEEK_FLAG_FLUSH | GST_SEEK_FLAG_ACCURATE, 2 * GST_SECOND)) return 9;
  if (!wait_state(player, GST_STATE_PAUSED)) return 10;
  gst_element_query_position(player, GST_FORMAT_TIME, &sought);
  if (sought < 1900 * GST_MSECOND) return 11;
  gst_element_set_state(player, GST_STATE_PLAYING);
  if (!wait_state(player, GST_STATE_PLAYING)) return 12;
  gint64 deadline = g_get_monotonic_time() + 10000000;
  gboolean eos = FALSE;
  while (g_get_monotonic_time() < deadline) {
    if (errors(bus)) return 13;
    GstMessage *message = gst_bus_timed_pop_filtered(bus, 100 * GST_MSECOND, GST_MESSAGE_EOS);
    if (message) { eos = TRUE; gst_message_unref(message); break; }
  }
  gst_element_set_state(player, GST_STATE_NULL);
  printf("MEDIA_RESULT: frames=%u changed=%u playing_ms=%lld paused_ms=%lld sought_ms=%lld eos=%d output=silent\n",
      frames, changed, (long long)(playing/GST_MSECOND), (long long)(paused/GST_MSECOND), (long long)(sought/GST_MSECOND), eos);
  printf("AUDIO_DECODE_RESULT: buffers=%d nonzero=%d output=silent\n",
      g_atomic_int_get(&audio_buffers), g_atomic_int_get(&nonzero_audio));
  gst_object_unref(bus); gst_object_unref(player);
  return eos && frames > 10 && changed > 5 && g_atomic_int_get(&nonzero_audio) > 10 ? 0 : 14;
}
