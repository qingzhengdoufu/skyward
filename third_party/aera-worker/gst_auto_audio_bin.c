// SPDX-License-Identifier: Apache-2.0
// WebKit expects autoaudiosink to expose GstChildProxy, which GstBin implements.
#include <gst/gst.h>

typedef struct { GstBin parent; } SkywardAudioBin;
typedef struct { GstBinClass parent; } SkywardAudioBinClass;
G_DEFINE_TYPE(SkywardAudioBin, skyward_audio_bin, GST_TYPE_BIN)

static void skyward_audio_bin_init(SkywardAudioBin *self) {
  GError *error = NULL;
  // Clocked silent output is intentional until the device soundcard is supported.
  GstElement *output = gst_parse_bin_from_description(
      "audioconvert ! audioresample ! audio/x-raw,format=S16LE,layout=interleaved,rate=48000,channels=2 ! fakesink sync=true async=true", TRUE, &error);
  if (!output) {
    GST_ELEMENT_ERROR(self, CORE, MISSING_PLUGIN, ("Browser audio fallback is unavailable"),
        ("%s", error ? error->message : "missing conversion element"));
    g_clear_error(&error);
    return;
  }
  gst_bin_add(GST_BIN(self), output);
  GstPad *pad = gst_element_get_static_pad(output, "sink");
  if (pad) { gst_element_add_pad(GST_ELEMENT(self), gst_ghost_pad_new("sink", pad)); gst_object_unref(pad); }
  GST_OBJECT_FLAG_SET(self, GST_ELEMENT_FLAG_SINK);
}

static void skyward_audio_bin_class_init(SkywardAudioBinClass *klass) {
  gst_element_class_set_static_metadata(GST_ELEMENT_CLASS(klass), "Browser audio fallback", "Sink/Audio",
      "Clocked silent output while device audio is unavailable", "Skyward contributors");
}

static gboolean plugin_init(GstPlugin *plugin) {
  return gst_element_register(plugin, "autoaudiosink", GST_RANK_PRIMARY + 100, skyward_audio_bin_get_type());
}

#ifndef PACKAGE
#define PACKAGE "skyward-browser"
#endif
GST_PLUGIN_DEFINE(GST_VERSION_MAJOR, GST_VERSION_MINOR, aeraaudio,
    "Browser audio fallback bin", plugin_init, "1.0", "Apache-2.0", "Skyward", "https://github.com/qingzhengdoufu/skyward")
