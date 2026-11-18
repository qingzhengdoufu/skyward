const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');

const source = fs.readFileSync(path.join(__dirname, 'rtc_probe.hpp'), 'utf8');
const body = source.match(/kRtcCapabilityProbe\[\] = R"JS\(([\s\S]*?)\)JS"/)[1];
const interfaceBody = source.match(/kRtcInterfaceProbe\[\] = R"JS\(([\s\S]*?)\)JS"/)[1];

async function run(mode) {
  let closed = 0;
  let cleared = 0;
  const directions = [];
  class Peer {
    constructor(options) {
      assert.equal(JSON.stringify(options), '{"iceServers":[]}');
      if (mode === 'construct') throw new Error('private message must not escape');
    }
    addTransceiver(kind, options) { directions.push([kind, options.direction]); }
    createOffer() {
      if (mode === 'reject') return Promise.reject(new Error('secret'));
      if (mode === 'timeout') return new Promise(() => {});
      return Promise.resolve({sdp: mode === 'no-h264' ? 'VP8/90000' : 'H264/90000'});
    }
    close() { closed++; if (mode === 'close') throw new Error('close failure'); }
  }
  const context = vm.createContext({
    isSecureContext: true,
    RTCPeerConnection: mode === 'missing' ? undefined : Peer,
    RTCDataChannel: class {},
    MediaStreamTrack: class {},
    WebSocket: class {},
    setTimeout(callback, ms) {
      assert.equal(ms, 4000);
      if (mode === 'timeout') queueMicrotask(callback);
      return 17;
    },
    clearTimeout(timer) { if (timer !== undefined) assert.equal(timer, 17); cleared++; },
  });
  const result = await vm.runInContext('(async()=>{' + body + '})()', context);
  const flags = JSON.parse(JSON.stringify(result));
  assert.equal(Object.keys(flags).length, 8);
  assert.ok(Object.values(flags).every(value => typeof value === 'boolean'));
  assert.equal(cleared, 1);
  assert.equal(flags.h264, ['ok', 'close'].includes(mode));
  assert.equal(flags.offer, ['ok', 'close', 'no-h264'].includes(mode));
  assert.equal(flags.timeout, mode === 'timeout');
  assert.equal(closed, ['missing', 'construct'].includes(mode) ? 0 : 1);
  if (!['missing', 'construct'].includes(mode))
    assert.deepEqual(directions, [['video', 'recvonly'], ['audio', 'recvonly']]);
  return flags;
}

(async () => {
  const interfaces = await vm.runInNewContext('(async()=>{' + interfaceBody + '})()', {
    isSecureContext: true,
    RTCPeerConnection: class { constructor() { throw new Error('must not create peer'); } },
    RTCDataChannel: class {}, MediaStreamTrack: class {}, WebSocket: class {},
  });
  assert.equal(interfaces.rtc, true);
  assert.equal(interfaces.h264, false);
  assert.equal(interfaces.offer, false);
  process.stdout.write('PASS interface probe creates no peer\n');
  for (const mode of ['ok', 'missing', 'construct', 'reject', 'timeout', 'no-h264', 'close']) {
    await run(mode);
    process.stdout.write('PASS capability probe ' + mode + '\n');
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
