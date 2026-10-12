// Spirit Collector audio: everything is synthesized with the Web Audio API, so there are no
// audio files to ship. Rust only sends two things: one-shot events  gameAudio.sfx(name)
// and the mood of the world  gameAudio.ambience(night, rain, fog).
//
// Cozy recipe: soft sine "music box" bells, a little reverb, gentle noise layers for wind / rain /
// fog, crickets at night, and a slow pentatonic melody that never resolves in a hurry.
(function () {
  'use strict';

  const mtof = (m) => 440 * Math.pow(2, (m - 69) / 12);
  const rnd = (a, b) => a + Math.random() * (b - a);
  const pick = (arr) => arr[Math.floor(Math.random() * arr.length)];

  function makeImpulse(ctx, seconds, decay) {
    const len = Math.floor(ctx.sampleRate * seconds);
    const buf = ctx.createBuffer(2, len, ctx.sampleRate);
    for (let c = 0; c < 2; c++) {
      const d = buf.getChannelData(c);
      for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / len, decay);
    }
    return buf;
  }

  function makeNoise(ctx, seconds) {
    const len = Math.floor(ctx.sampleRate * seconds);
    const buf = ctx.createBuffer(1, len, ctx.sampleRate);
    const d = buf.getChannelData(0);
    let b0 = 0, b1 = 0, b2 = 0; // gentle pink-ish filtering so it is warm, not hissy
    for (let i = 0; i < len; i++) {
      const w = Math.random() * 2 - 1;
      b0 = 0.99765 * b0 + w * 0.099046;
      b1 = 0.963 * b1 + w * 0.2965164;
      b2 = 0.57 * b2 + w * 1.0526913;
      d[i] = (b0 + b1 + b2 + w * 0.1848) * 0.2;
    }
    return buf;
  }

  function createGameAudio(ctx, opts) {
    opts = opts || {};
    const MASTER = 0.55;
    let muted = false;
    try { muted = window.localStorage.getItem('sc-muted') === '1'; } catch (e) { /* storage blocked */ }
    const mood = { night: 0, rain: 0, fog: 0 };

    // ---------------------------------------------------------------- buses
    const master = ctx.createGain();
    master.gain.value = muted ? 0 : MASTER;
    const comp = ctx.createDynamicsCompressor();
    comp.threshold.value = -20; comp.ratio.value = 4; comp.attack.value = 0.01; comp.release.value = 0.25;
    master.connect(comp); comp.connect(ctx.destination);

    const verb = ctx.createConvolver();
    verb.buffer = makeImpulse(ctx, 2.6, 2.8);
    const verbGain = ctx.createGain(); verbGain.gain.value = 0.38;
    verb.connect(verbGain); verbGain.connect(master);

    // Player volume sliders (0..1). Master scales everything; the rest scale their own bus.
    const vol = { master: 1, music: 1, sfx: 1, ambience: 1 };
    const AMB_BASE = 0.9;
    const dry = ctx.createGain(); dry.connect(master);
    const sfxBus = ctx.createGain(); sfxBus.connect(dry); sfxBus.connect(verb);
    // Music gets its own volume stage after the mood-driven bus, so the two never fight.
    const musicVol = ctx.createGain(); musicVol.connect(dry); musicVol.connect(verb);
    const musicBus = ctx.createGain(); musicBus.gain.value = 0.55; musicBus.connect(musicVol);
    const ambBus = ctx.createGain(); ambBus.connect(master);

    const noiseBuf = makeNoise(ctx, 3);

    // ---------------------------------------------------------------- voices
    function bell(freq, t, vol, dur, dest) {
      const g = ctx.createGain();
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(vol, t + 0.006);
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      g.connect(dest || sfxBus);
      [[1, 1], [2.01, 0.26], [3.97, 0.08]].forEach(([mul, amp]) => {
        const o = ctx.createOscillator();
        o.type = 'sine';
        o.frequency.value = freq * mul;
        const og = ctx.createGain(); og.gain.value = amp;
        o.connect(og); og.connect(g);
        o.start(t); o.stop(t + dur + 0.05);
      });
    }

    function blip(type, f0, f1, t, dur, vol, dest) {
      const o = ctx.createOscillator(), g = ctx.createGain();
      o.type = type;
      o.frequency.setValueAtTime(f0, t);
      o.frequency.exponentialRampToValueAtTime(Math.max(1, f1), t + dur);
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(vol, t + 0.012);
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      o.connect(g); g.connect(dest || sfxBus);
      o.start(t); o.stop(t + dur + 0.05);
    }

    function puff(t, dur, vol, freq, q, dest) {
      const s = ctx.createBufferSource(); s.buffer = noiseBuf;
      const f = ctx.createBiquadFilter(); f.type = 'bandpass'; f.frequency.value = freq; f.Q.value = q;
      const g = ctx.createGain();
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(vol, t + Math.min(0.02, dur * 0.3));
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      s.connect(f); f.connect(g); g.connect(dest || sfxBus);
      s.start(t, rnd(0, 1.5), dur + 0.05);
    }

    // Slow-attack chord pad.
    function swell(midis, t, dur, vol, dest) {
      const g = ctx.createGain();
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(vol, t + dur * 0.45);
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 1400;
      g.connect(lp); lp.connect(dest || sfxBus);
      midis.forEach((m, i) => {
        const o = ctx.createOscillator();
        o.type = i % 2 ? 'triangle' : 'sine';
        o.frequency.value = mtof(m);
        o.detune.value = rnd(-6, 6);
        o.connect(g); o.start(t); o.stop(t + dur + 0.05);
      });
    }

    // ---------------------------------------------------------------- one-shot effects
    const SFX = {
      step(t) { puff(t, 0.08, 0.17, rnd(520, 820), 0.8); },
      pickup(t) { bell(mtof(79), t, 0.16, 1.0); bell(mtof(86), t + 0.07, 0.13, 1.2); },
      drop(t) { blip('sine', 240, 110, t, 0.14, 0.22); puff(t, 0.08, 0.08, 500, 0.7); },
      munch(t) {
        [0, 0.19, 0.38].forEach((d) => { puff(t + d, 0.09, 0.14, rnd(380, 520), 1.2); blip('sine', 300, 220, t + d, 0.06, 0.05); });
      },
      befriend(t) {
        [72, 76, 79, 84, 88].forEach((m, i) => bell(mtof(m), t + i * 0.09, 0.17, 1.8));
        puff(t + 0.45, 0.5, 0.04, 6000, 2);
      },
      deposit(t) {
        bell(mtof(48), t, 0.17, 2.4);
        [72, 79, 84, 88, 91].forEach((m, i) => bell(mtof(m), t + 0.05 + i * 0.1, 0.15, 1.8));
      },
      buy(t) { bell(mtof(88), t, 0.14, 0.9); bell(mtof(95), t + 0.08, 0.12, 1.3); },
      lantern(t) {
        bell(mtof(60), t, 0.22, 2.2); bell(mtof(67), t + 0.06, 0.14, 2.0);
        puff(t, 0.5, 0.06, 1200, 0.6);
      },
      puzzle(t) {
        [60, 64, 67, 72].forEach((m, i) => bell(mtof(m), t + i * 0.16, 0.18, 2.8));
        swell([48, 55, 60, 64], t, 3.4, 0.07);
      },
      spot(t) { blip('sine', 520, 820, t, 0.38, 0.06); },
      flee(t) { blip('triangle', 720, 300, t, 0.24, 0.09); },
      nope(t) { blip('sine', 230, 220, t, 0.12, 0.10); blip('sine', 200, 190, t + 0.12, 0.16, 0.09); },
      ui(t) { bell(mtof(96), t, 0.07, 0.25); },
      // Journal: a quick pencil scratch, then a tiny bell.
      clue(t) {
        puff(t, 0.16, 0.05, 3400, 1.4);
        puff(t + 0.09, 0.14, 0.045, 4200, 1.4);
        bell(mtof(88), t + 0.22, 0.10, 1.4);
        bell(mtof(95), t + 0.30, 0.08, 1.6);
      },
      // Graveyard Hollow: the four rune stones each have their own bell, and the gate creaks.
      rune0(t) { bell(mtof(64), t, 0.20, 1.8); },
      rune1(t) { bell(mtof(67), t, 0.20, 1.8); },
      rune2(t) { bell(mtof(71), t, 0.20, 1.8); },
      rune3(t) { bell(mtof(74), t, 0.20, 1.8); },
      gate(t) {
        blip('triangle', 150, 85, t, 0.55, 0.10);
        puff(t, 0.5, 0.07, 700, 1.4);
        swell([45, 52, 57], t + 0.1, 2.6, 0.06);
        bell(mtof(81), t + 0.5, 0.10, 2.0);
      },
      // Weather cues: gentle, but you will notice them.
      weather_clear(t) { [76, 79, 84].forEach((m, i) => bell(mtof(m), t + i * 0.14, 0.12, 2.0)); },
      weather_fog(t) {
        swell([50, 57, 62], t, 3.2, 0.06);
        bell(mtof(62), t + 0.2, 0.10, 2.4);
      },
      weather_rain(t) {
        [79, 76, 74, 69].forEach((m, i) => bell(mtof(m), t + i * 0.16, 0.11, 1.6));
        puff(t, 1.6, 0.06, 3500, 0.5);
      },
    };

    // ---------------------------------------------------------------- ambience layers
    function loopNoise() {
      const s = ctx.createBufferSource(); s.buffer = noiseBuf; s.loop = true; s.start(0, rnd(0, 2));
      return s;
    }
    function lfo(freq, depth, target) {
      const o = ctx.createOscillator(); o.frequency.value = freq;
      const g = ctx.createGain(); g.gain.value = depth;
      o.connect(g); g.connect(target); o.start(0);
      return o;
    }

    // wind
    const windG = ctx.createGain(); windG.gain.value = 0;
    const windLP = ctx.createBiquadFilter(); windLP.type = 'lowpass'; windLP.frequency.value = 420;
    loopNoise().connect(windLP); windLP.connect(windG); windG.connect(ambBus);
    lfo(0.07, 160, windLP.frequency);

    // rain
    const rainG = ctx.createGain(); rainG.gain.value = 0;
    const rainHP = ctx.createBiquadFilter(); rainHP.type = 'highpass'; rainHP.frequency.value = 900;
    const rainLP = ctx.createBiquadFilter(); rainLP.type = 'lowpass'; rainLP.frequency.value = 7000;
    loopNoise().connect(rainHP); rainHP.connect(rainLP); rainLP.connect(rainG); rainG.connect(ambBus);

    // fog pad (low, slow, a little eerie)
    const fogG = ctx.createGain(); fogG.gain.value = 0;
    const fogLP = ctx.createBiquadFilter(); fogLP.type = 'lowpass'; fogLP.frequency.value = 520;
    [110, 164.8, 221.2].forEach((f, i) => {
      const o = ctx.createOscillator(); o.type = 'sine'; o.frequency.value = f; o.detune.value = (i - 1) * 7;
      o.connect(fogLP); o.start(0);
    });
    fogLP.connect(fogG); fogG.connect(ambBus);
    lfo(0.09, 0.012, fogG.gain);

    // crickets: two tiny voices chirping in gated bursts
    const cricG = ctx.createGain(); cricG.gain.value = 0; cricG.connect(ambBus);
    [[4200, 15, 0.7], [4750, 17.5, 0.55]].forEach(([f, chirpHz, burstHz]) => {
      const o = ctx.createOscillator(); o.type = 'sine'; o.frequency.value = f;
      const gate1 = ctx.createGain(); gate1.gain.value = 0.5; lfo(chirpHz, 0.5, gate1.gain);
      const gate2 = ctx.createGain(); gate2.gain.value = 0.5; lfo(burstHz, 0.5, gate2.gain);
      o.connect(gate1); gate1.connect(gate2); gate2.connect(cricG); o.start(0);
    });

    function setTarget(param, value, tc) {
      const now = ctx.currentTime;
      param.cancelScheduledValues(now);
      param.setTargetAtTime(value, now, tc || 2.0);
    }

    const api = {
      ctx,
      sfx(name, when) {
        const fn = SFX[name];
        if (!fn) return;
        fn(when === undefined ? ctx.currentTime + 0.005 : when);
      },
      // night / rain / fog are each 0..1
      ambience(night, rain, fog) {
        mood.night = night; mood.rain = rain; mood.fog = fog;
        setTarget(windG.gain, 0.05 + 0.06 * (1 - night) + 0.03 * rain);
        setTarget(rainG.gain, 0.17 * rain);
        setTarget(fogG.gain, 0.032 * fog);
        setTarget(cricG.gain, 0.022 * night * (1 - rain) * (1 - 0.6 * fog));
        setTarget(musicBus.gain, 0.55 * (1 - 0.35 * rain) * (1 - 0.25 * night));
      },
      // Player volume sliders, each 0..1: { master, music, sfx, ambience }.
      setVolumes(v) {
        ['master', 'music', 'sfx', 'ambience'].forEach((k) => {
          if (v && typeof v[k] === 'number' && isFinite(v[k])) vol[k] = Math.max(0, Math.min(1, v[k]));
        });
        setTarget(master.gain, muted ? 0 : MASTER * vol.master, 0.05);
        setTarget(sfxBus.gain, vol.sfx, 0.05);
        setTarget(musicVol.gain, vol.music, 0.05);
        setTarget(ambBus.gain, AMB_BASE * vol.ambience, 0.05);
      },
      // A spirit is close. near: 0..1 (1 = right here), pan: -1..1 (left/right of you), note: which
      // spirit (each species has its own pitch, so you can learn them by ear), fresh: never befriended.
      sense(near, pan, note, fresh) {
        const SCALE = [79, 81, 84, 86, 88, 91, 93, 96];
        const n = Math.max(0, Math.floor(note || 0));
        const f = mtof(SCALE[n % SCALE.length]);
        const level = (0.035 + 0.075 * Math.max(0, Math.min(1, near || 0))) * (fresh ? 1 : 0.55);
        const t = ctx.currentTime + 0.01;
        let dest = sfxBus;
        if (ctx.createStereoPanner) {
          const p = ctx.createStereoPanner();
          p.pan.value = Math.max(-1, Math.min(1, pan || 0));
          p.connect(sfxBus);
          dest = p;
        }
        // Two quick glints a fifth apart, like a wind chime brushed once.
        bell(f, t, level, 1.3, dest);
        bell(f * 1.5, t + 0.11, level * 0.7, 1.1, dest);
        if (fresh) bell(f * 2, t + 0.24, level * 0.45, 1.0, dest);
      },
      setMuted(m) {
        muted = !!m;
        try { window.localStorage.setItem('sc-muted', muted ? '1' : '0'); } catch (e) { /* ignore */ }
        setTarget(master.gain, muted ? 0 : MASTER * vol.master, 0.08);
      },
      toggleMute() { api.setMuted(!muted); return muted; },
      get muted() { return muted; },
      // ---- music: a slow pentatonic wander. Called on a timer; plays at most one note per call.
      _nextNote: 0,
      _idx: 2,
      musicTick() {
        if (muted) return;
        const t = ctx.currentTime;
        if (t < api._nextNote) return;
        api._nextNote = t + rnd(1.6, 4.2);
        if (Math.random() < 0.22) return; // breathing room
        const day = [72, 74, 76, 79, 81, 84];
        const nightScale = [69, 72, 74, 76, 79, 81];
        const scale = mood.night > 0.5 ? nightScale : day;
        api._idx = Math.max(0, Math.min(scale.length - 1, api._idx + pick([-2, -1, -1, 0, 1, 1, 2])));
        const lower = mood.rain > 0.5 ? -12 : 0;
        bell(mtof(scale[api._idx] + lower), t + 0.02, 0.11, 3.0, musicBus);
        if (Math.random() < 0.15) bell(mtof(scale[Math.max(0, api._idx - 2)] + lower - 12), t + 0.3, 0.07, 3.4, musicBus);
        if (Math.random() < 0.07) swell([mood.night > 0.5 ? 45 : 48, mood.night > 0.5 ? 52 : 55], t, 7, 0.035, musicBus);
      },
    };

    api.setVolumes({});
    if (opts.auto !== false) {
      window.setInterval(() => { if (ctx.state === 'running') api.musicTick(); }, 300);
    }
    return api;
  }

  window.createGameAudio = createGameAudio;

  // Real context for the game. Browsers keep audio suspended until the player does something.
  try {
    const AC = window.AudioContext || window.webkitAudioContext;
    if (AC) {
      const ctx = new AC();
      const audio = createGameAudio(ctx);
      window.gameAudio = audio;
      const unlock = () => { if (ctx.state !== 'running') ctx.resume(); };
      ['keydown', 'pointerdown', 'touchstart'].forEach((ev) => window.addEventListener(ev, unlock, { passive: true }));
      window.addEventListener('keydown', (e) => {
        if (e.code === 'KeyM' && !e.repeat) {
          const m = audio.toggleMute();
          let n = document.getElementById('audio-note');
          if (!n) {
            n = document.createElement('div');
            n.id = 'audio-note';
            n.style.cssText = 'position:fixed;left:50%;bottom:90px;transform:translateX(-50%);padding:6px 16px;' +
              'background:#1c1530;box-shadow:0 0 0 3px #1c1530,0 0 0 6px #e8b45a;color:#f6e6c4;font:20px VT323,monospace;' +
              'transition:opacity .4s;pointer-events:none;z-index:5';
            document.body.appendChild(n);
          }
          n.textContent = m ? '🔇 Sound off  (M)' : '🔊 Sound on  (M)';
          n.style.opacity = '1';
          window.clearTimeout(n._t);
          n._t = window.setTimeout(() => { n.style.opacity = '0'; }, 1600);
        }
      });
    }
  } catch (e) {
    console.warn('Audio unavailable:', e);
  }
})();
