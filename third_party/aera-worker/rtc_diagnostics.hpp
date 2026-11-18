// SPDX-License-Identifier: Apache-2.0
#pragma once

// Private cloud-site diagnostic: no SDP, candidates, URLs, credentials or text.
inline constexpr const char kRtcDiagnostics[] = R"JS((()=>{
  if(location.hostname!=='ys.mihoyo.com'||typeof RTCPeerConnection!=='function')return;
  const Native=RTCPeerConnection;
  let peers=0, messages=0;
  const send=(...parts)=>{
    try{if(messages++<256)window.webkit.messageHandlers.skywardRtc.postMessage(parts.join(' '));}catch(_){}
  };
  const number=value=>Number.isFinite(value)?Math.max(0,Math.min(1e12,Math.floor(value))):0;
  const name=value=>typeof value==='string'&&/^[A-Za-z0-9_-]{1,32}$/.test(value)?value:'unknown';
  const observe=peer=>{
    const id=++peers;
    if(id>32)return;
    send(id,'created');
    const state=()=>send(id,'state',name(peer.connectionState),name(peer.iceConnectionState),name(peer.iceGatheringState),name(peer.signalingState));
    for(const event of ['connectionstatechange','iceconnectionstatechange','icegatheringstatechange','signalingstatechange'])
      peer.addEventListener(event,state);
    peer.addEventListener('icecandidateerror',event=>send(id,'ice-error',number(event.errorCode)));
    peer.addEventListener('track',event=>send(id,'track',name(event.track?.kind)));
    peer.addEventListener('datachannel',event=>{
      const channel=event.channel;
      send(id,'channel',name(channel.readyState));
      for(const event of ['open','close','error'])channel.addEventListener(event,()=>send(id,'channel',name(channel.readyState)));
    });
    for(const method of ['setLocalDescription','setRemoteDescription','addIceCandidate']){
      const native=peer[method];
      if(typeof native!=='function')continue;
      peer[method]=function(...args){
        try{
          const result=Reflect.apply(native,this,args);
          if(result&&typeof result.then==='function')result.then(()=>send(id,method,'ok'),error=>send(id,method,name(error?.name)));
          return result;
        }catch(error){send(id,method,name(error?.name));throw error;}
      };
    }
    let polls=0,busy=false;
    const timer=setInterval(async()=>{
      if(peer.connectionState==='closed'||++polls>30){clearInterval(timer);return;}
      if(busy)return;
      busy=true;
      try{
        const stats=await peer.getStats();
        let pairs=0,sent=0,received=0,packets=0,frames=0;
        stats.forEach(item=>{
          if(item.type==='candidate-pair'&&item.state==='succeeded'){
            pairs++;sent+=number(item.bytesSent);received+=number(item.bytesReceived);
          }
          if(item.type==='inbound-rtp'){
            packets+=number(item.packetsReceived);frames+=number(item.framesDecoded);
          }
          if(item.type==='transport')send(id,'transport',name(item.dtlsState));
        });
        send(id,'stats',number(pairs),number(sent),number(received),number(packets),number(frames));
      }catch(error){send(id,'stats-error',name(error?.name));}
      finally{busy=false;}
    },3000);
    window.addEventListener('pagehide',()=>clearInterval(timer),{once:true});
    state();
  };
  window.RTCPeerConnection=new Proxy(Native,{
    construct(target,args,newTarget){
      const peer=Reflect.construct(target,args,newTarget);
      try{observe(peer);}catch(_){}
      return peer;
    }
  });
})();)JS";
