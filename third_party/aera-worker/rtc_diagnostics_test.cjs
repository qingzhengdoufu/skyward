const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, 'rtc_diagnostics.hpp'), 'utf8');
const script = source.match(/R"JS\(([\s\S]*?)\)JS"/)[1];

function setup(host = 'ys.mihoyo.com', throwOnLog = false) {
  const logs = [], timers = new Map(), events = {};
  let nextTimer = 0, calls = 0;
  class Peer {
    constructor(options) {
      if (options.fail) throw new TypeError('secret constructor');
      this.options = options;
      this.connectionState = 'new'; this.iceConnectionState = 'new';
      this.iceGatheringState = 'new'; this.signalingState = 'stable';
      this.events = {};
    }
    addEventListener(name, callback) { this.events[name] = callback; }
    setLocalDescription(value) { calls++; this.value = value; return Promise.resolve(); }
    setRemoteDescription() { return Promise.reject(new TypeError('secret SDP and token')); }
    addIceCandidate() { throw new TypeError('secret candidate'); }
    getStats() {
      return Promise.resolve(new Map([
        ['pair', {type: 'candidate-pair', state: 'succeeded', bytesSent: 10, bytesReceived: 20, address: 'secret address'}],
        ['inbound', {type: 'inbound-rtp', packetsReceived: 4, framesDecoded: 3}],
        ['transport', {type: 'transport', dtlsState: 'connected'}],
      ]));
    }
  }
  const window = {
    RTCPeerConnection: Peer,
    webkit: {messageHandlers: {skywardRtc: {postMessage(text) {
      if (throwOnLog) throw Error('bridge missing');
      logs.push(text);
    }}}},
    addEventListener(name, callback) { (events[name] ||= []).push(callback); },
  };
  const context = vm.createContext({window, location: {hostname: host}, RTCPeerConnection: Peer,
    setInterval(callback) { timers.set(++nextTimer, callback); return nextTimer; },
    clearInterval(id) { timers.delete(id); },
  });
  vm.runInContext(script, context);
  return {window, logs, timers, events, Peer, calls: () => calls};
}

(async () => {
  const other = setup('example.com');
  assert.equal(other.window.RTCPeerConnection, other.Peer);
  assert.equal(other.logs.length, 0);
  const test = setup();
  const Peer = test.window.RTCPeerConnection;
  const options = {iceServers: [{urls: 'stun:secret.example'}]};
  const peer = new Peer(options);
  assert.ok(peer instanceof test.Peer);
  assert.equal(Peer.prototype, test.Peer.prototype);
  assert.equal(peer.options, options);
  assert.throws(() => new Peer({fail: true}), TypeError);
  const offer = {type: 'offer', sdp: 'secret SDP'};
  await peer.setLocalDescription(offer);
  assert.equal(peer.value, offer);
  assert.equal(test.calls(), 1);
  await assert.rejects(peer.setRemoteDescription({}), TypeError);
  assert.throws(() => peer.addIceCandidate({}), TypeError);
  peer.events.icecandidateerror({errorCode: 701, url: 'secret URL', errorText: 'secret text'});
  await [...test.timers.values()][0]();
  assert.ok(test.logs.includes('1 stats 1 10 20 4 3'));
  assert.ok(test.logs.includes('1 transport connected'));
  assert.ok(test.logs.includes('1 ice-error 701'));
  assert.ok(test.logs.every(text => text.length <= 256 && /^[A-Za-z0-9 _-]+$/.test(text)));
  assert.ok(!test.logs.join('\n').includes('secret'));
  for (let i = 0; i < 600; i++) peer.events.connectionstatechange();
  assert.equal(test.logs.length, 256);
  test.events.pagehide.forEach(callback => callback());
  assert.equal(test.timers.size, 0);
  const broken = setup('ys.mihoyo.com', true);
  const withBrokenLog = new broken.window.RTCPeerConnection({});
  await withBrokenLog.setLocalDescription(offer);
  assert.equal(broken.calls(), 1);
  console.log('PASS RTC diagnostics: host gate, native semantics, failure isolation, redaction, bounds and cleanup');
})().catch(error => { console.error(error); process.exitCode = 1; });
