// SPDX-License-Identifier: Apache-2.0
#pragma once

// Run in an isolated script world; never read page state or connect peers.
inline constexpr const char kRtcInterfaceProbe[] = R"JS(
return {
  secure: isSecureContext,
  rtc: typeof RTCPeerConnection === 'function',
  datachannel: typeof RTCDataChannel === 'function',
  websocket: typeof WebSocket === 'function',
  mediatrack: typeof MediaStreamTrack === 'function',
  h264: false,
  offer: false,
  timeout: false
};
)JS";

inline constexpr const char kRtcCapabilityProbe[] = R"JS(
const result = {
  secure: isSecureContext,
  rtc: typeof RTCPeerConnection === 'function',
  datachannel: typeof RTCDataChannel === 'function',
  websocket: typeof WebSocket === 'function',
  mediatrack: typeof MediaStreamTrack === 'function',
  h264: false,
  offer: false,
  timeout: false
};
let peer;
let timer;
try {
  if (result.rtc && result.datachannel && result.mediatrack) {
    peer = new RTCPeerConnection({iceServers: []});
    peer.addTransceiver('video', {direction: 'recvonly'});
    peer.addTransceiver('audio', {direction: 'recvonly'});
    const timeout = new Promise(resolve => {
      timer = setTimeout(() => { result.timeout = true; resolve(null); }, 4000);
    });
    const offer = await Promise.race([peer.createOffer(), timeout]);
    result.offer = !!offer;
    result.h264 = typeof offer?.sdp === 'string' && offer.sdp.includes('H264/90000');
  }
} catch (_) {
  // Diagnostics must not fail the browser or capture exception text from a site.
} finally {
  clearTimeout(timer);
  if (peer) { try { peer.close(); } catch (_) {} }
}
return result;
)JS";
