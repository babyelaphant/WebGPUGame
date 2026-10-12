// Title screen, pause menu and settings. Plain DOM, no dependencies.
//
// This file owns the menu UI and the player's settings (kept in localStorage). The Rust game talks to it
// through `window.gameMenu` (see the bottom): it asks whether a menu is open (the world pauses while one
// is), picks up actions like "continue" / "new" / "quit", and re-reads the settings text whenever
// `settingsVersion()` changes. The settings text is the same `key=value` format as src/settings.rs.
(function () {
  'use strict';

  const GAME_NAME = 'untitled-01'; // placeholder title, change it here
  const TAGLINE = 'a cozy little ghost-collecting game';
  const SETTINGS_KEY = 'untitled-01-settings-v1';

  // ---------------------------------------------------------------- settings model
  // [default, min, max]. Toggles are 0/1.
  const SPEC = {
    master: [10, 0, 10], music: [10, 0, 10], sfx: [10, 0, 10], ambience: [10, 0, 10],
    brightness: [10, 5, 15], ui_scale: [1, 0, 2], pixel: [1, 0, 2], weather_fx: [2, 0, 2],
    vignette: [1, 0, 1], hints: [1, 0, 1], shimmer: [1, 0, 1], fps: [0, 0, 1], run_toggle: [0, 0, 1],
  };
  const KEYS = Object.keys(SPEC);
  const clamp = (n, lo, hi) => Math.max(lo, Math.min(hi, n));

  function defaults() {
    const o = {};
    KEYS.forEach((k) => { o[k] = SPEC[k][0]; });
    return o;
  }

  function parse(text) {
    const out = defaults();
    String(text || '').split('\n').forEach((line) => {
      const i = line.indexOf('=');
      if (i < 0) return;
      const k = line.slice(0, i).trim();
      const n = parseInt(line.slice(i + 1).trim(), 10);
      if (SPEC[k] && Number.isFinite(n)) out[k] = clamp(n, SPEC[k][1], SPEC[k][2]);
    });
    return out;
  }

  function serialize(s) {
    return KEYS.map((k) => k + '=' + s[k]).join('\n') + '\n';
  }

  let settings = defaults();
  try {
    const t = window.localStorage.getItem(SETTINGS_KEY);
    if (t) settings = parse(t);
  } catch (e) { /* storage blocked: use defaults */ }
  let version = 1;

  // ---------------------------------------------------------------- applying settings
  function applySettings() {
    const a = window.gameAudio;
    if (a && a.setVolumes) {
      a.setVolumes({
        master: settings.master / 10, music: settings.music / 10,
        sfx: settings.sfx / 10, ambience: settings.ambience / 10,
      });
    }
    const canvas = document.getElementById('gpu');
    if (canvas) canvas.style.filter = settings.brightness === 10 ? '' : 'brightness(' + (settings.brightness / 10) + ')';
    const hud = document.getElementById('hud');
    if (hud) hud.style.zoom = String([0.88, 1, 1.18][settings.ui_scale]);
    document.body.classList.toggle('no-hints', !settings.hints);
  }

  function change(key, value) {
    const [, lo, hi] = SPEC[key];
    const v = clamp(Math.round(value), lo, hi);
    if (settings[key] === v) return false;
    settings[key] = v;
    version++;
    persist();
    applySettings();
    return true;
  }

  function persist() {
    try { window.localStorage.setItem(SETTINGS_KEY, serialize(settings)); } catch (e) { /* ignore */ }
  }

  // ---------------------------------------------------------------- the rows in the settings menu
  const isFs = () => !!(document.fullscreenElement || document.webkitFullscreenElement);
  function setFs(on) {
    try {
      const el = document.documentElement;
      if (on) {
        const p = (el.requestFullscreen || el.webkitRequestFullscreen || function () {}).call(el);
        if (p && p.catch) p.catch(() => {});
      } else {
        const p = (document.exitFullscreen || document.webkitExitFullscreen || function () {}).call(document);
        if (p && p.catch) p.catch(() => {});
      }
    } catch (e) { /* not allowed right now */ }
  }
  const soundOn = () => !(window.gameAudio && window.gameAudio.muted);
  function setSound(on) {
    const a = window.gameAudio;
    if (a && a.setMuted) a.setMuted(!on);
  }

  const TABS = [
    {
      name: 'Audio',
      rows: [
        { key: 'master', label: 'Master volume', type: 'slider' },
        { key: 'music', label: 'Music', type: 'slider' },
        { key: 'sfx', label: 'Sound effects', type: 'slider' },
        { key: 'ambience', label: 'Ambience (wind, rain, crickets)', type: 'slider' },
        { label: 'Sound', type: 'toggle', get: soundOn, set: setSound, on: 'On', off: 'Muted' },
      ],
    },
    {
      name: 'Video',
      rows: [
        { label: 'Fullscreen', type: 'toggle', get: isFs, set: setFs },
        { key: 'pixel', label: 'Pixel size', note: 'Chunky is the most cozy-retro', type: 'choice', options: ['Chunky', 'Normal', 'Fine'] },
        { key: 'brightness', label: 'Brightness', type: 'slider', show: (v) => v * 10 + '%' },
        { key: 'weather_fx', label: 'Weather effects', note: 'Rain streaks and fog banks', type: 'choice', options: ['Off', 'Subtle', 'Full'] },
        { key: 'vignette', label: 'Dark screen edges', type: 'toggle' },
        { key: 'ui_scale', label: 'Interface size', type: 'choice', options: ['Small', 'Normal', 'Large'] },
        { key: 'fps', label: 'Show FPS', type: 'toggle' },
      ],
    },
    {
      name: 'Gameplay',
      rows: [
        { key: 'hints', label: 'Hints & tutorial text', type: 'toggle' },
        { key: 'shimmer', label: 'Spirit shimmer & compass', note: 'Sparkles over nearby spirits and a hint toward them', type: 'toggle' },
        { key: 'run_toggle', label: 'Running', type: 'choice', options: ['Hold Shift', 'Press Shift to toggle'] },
        { label: 'Reset all settings', type: 'button', run: resetAll },
      ],
    },
    { name: 'Controls', rows: [], controls: true },
  ];

  const CONTROLS = [
    ['Move', 'W A S D / Arrow keys'],
    ['Run', 'Shift'],
    ['Interact: gather, light, open, touch', 'E / Enter'],
    ['Put down the selected treat', 'Space'],
    ['Change treat', 'Q / R / Tab'],
    ['Spirit journal (A / D turns the page)', 'J'],
    ['Buy upgrades at the cottage', '1 – 5'],
    ['Sound on / off', 'M'],
    ['Pause menu', 'Esc'],
    ['Debug info', 'F1'],
  ];

  function resetAll() {
    KEYS.forEach((k) => { settings[k] = SPEC[k][0]; });
    version++;
    persist();
    applySettings();
  }

  function rowValue(row) {
    if (row.get) return row.get() ? 1 : 0;
    return settings[row.key];
  }

  function rowBounds(row) {
    if (row.type === 'toggle') return [0, 1];
    if (row.type === 'choice') return [0, row.options.length - 1];
    return [SPEC[row.key][1], SPEC[row.key][2]];
  }

  function setRow(row, v) {
    const [lo, hi] = rowBounds(row);
    v = clamp(v, lo, hi);
    if (row.set) { row.set(!!v); return true; }
    if (row.key) return change(row.key, v);
    return false;
  }

  // ---------------------------------------------------------------- menu state machine
  let screen = 'title'; // null | 'title' | 'pause' | 'settings' | 'confirm'
  let from = 'title'; // where Settings goes back to
  let playing = false; // a game is running behind the menu
  let loading = true; // the wasm game is still starting
  let hasSave = false;
  let saveSummary = '';
  let tab = 0;
  let focus = 0;
  const actions = [];
  const root = () => document.getElementById('menu');

  function sfx(name) {
    const a = window.gameAudio;
    if (a && a.sfx) { try { a.sfx(name); } catch (e) { /* audio not ready */ } }
  }

  function items() {
    if (screen === 'title') {
      const list = [];
      list.push({ act: 'continue', label: hasSave ? 'Continue' : 'Play', note: hasSave ? saveSummary : '' });
      if (hasSave) list.push({ act: 'new', label: 'New game' });
      list.push({ act: 'settings', label: 'Settings' });
      return list;
    }
    if (screen === 'pause') {
      return [
        { act: 'resume', label: 'Resume' },
        { act: 'settings', label: 'Settings' },
        { act: 'quit', label: 'Save & back to title' },
      ];
    }
    if (screen === 'confirm') {
      return [
        { act: 'cancel', label: 'Keep my game' },
        { act: 'erase', label: 'Erase it and start over' },
      ];
    }
    return [];
  }

  function rowCount() {
    return screen === 'settings' ? TABS[tab].rows.length : items().length;
  }

  function setScreen(next) {
    screen = next;
    focus = 0;
    document.body.classList.toggle('on-title', screen !== null && !playing);
    document.body.classList.toggle('menu-open', screen !== null);
    render();
  }

  function close() { setScreen(null); }

  function activate(act) {
    if (loading && (act === 'continue' || act === 'new' || act === 'erase')) return;
    sfx('ui');
    switch (act) {
      case 'continue':
        actions.push('continue'); playing = true; close(); break;
      case 'new':
        setScreen('confirm'); break;
      case 'erase':
        actions.push('new'); hasSave = false; playing = true; close(); break;
      case 'cancel':
        setScreen('title'); break;
      case 'settings':
        from = screen; tab = 0; setScreen('settings'); break;
      case 'resume':
        actions.push('resume'); close(); break;
      case 'quit':
        actions.push('quit'); playing = false; setScreen('title'); break;
      case 'back':
        setScreen(from); break;
      default: break;
    }
  }

  function back() {
    if (screen === 'settings') activate('back');
    else if (screen === 'pause') activate('resume');
    else if (screen === 'confirm') activate('cancel');
  }

  function adjust(dir) {
    if (screen !== 'settings') return;
    const row = TABS[tab].rows[focus];
    if (!row || row.type === 'button') return;
    const cur = rowValue(row);
    if (setRow(row, cur + dir)) sfx('ui');
    render();
  }

  function press() {
    if (screen === 'settings') {
      const row = TABS[tab].rows[focus];
      if (!row) return;
      if (row.type === 'button') { row.run(); sfx('buy'); render(); return; }
      const [lo, hi] = rowBounds(row);
      const cur = rowValue(row);
      if (row.type === 'toggle') setRow(row, cur ? 0 : 1);
      else if (row.type === 'choice') setRow(row, cur >= hi ? lo : cur + 1);
      sfx('ui');
      render();
      return;
    }
    const it = items()[focus];
    if (it) activate(it.act);
  }

  function moveFocus(d) {
    const n = rowCount();
    if (!n) return;
    focus = (focus + d + n) % n;
    sfx('ui');
    render();
  }

  function switchTab(d) {
    if (screen !== 'settings') return;
    tab = (tab + d + TABS.length) % TABS.length;
    focus = 0;
    sfx('ui');
    render();
  }

  function onKey(e) {
    if (screen === null) return; // the game owns the keyboard
    const c = e.code;
    if (c === 'KeyM') { window.setTimeout(render, 30); return; } // let the audio engine toggle sound
    let used = true;
    if (c === 'ArrowUp' || c === 'KeyW') moveFocus(-1);
    else if (c === 'ArrowDown' || c === 'KeyS') moveFocus(1);
    else if (c === 'ArrowLeft' || c === 'KeyA') adjust(-1);
    else if (c === 'ArrowRight' || c === 'KeyD') adjust(1);
    else if (c === 'Enter' || c === 'Space' || c === 'KeyE' || c === 'NumpadEnter') press();
    else if (c === 'Escape') back();
    else if (c === 'Tab') switchTab(e.shiftKey ? -1 : 1);
    else if (c === 'PageDown' || c === 'BracketRight') switchTab(1);
    else if (c === 'PageUp' || c === 'BracketLeft') switchTab(-1);
    else used = false;
    // While a menu is open the game never sees the keyboard.
    if (used) e.preventDefault();
    e.stopImmediatePropagation();
  }

  function onClick(e) {
    if (screen === null) return;
    const el = e.target && e.target.closest ? e.target.closest('[data-act],[data-tab],[data-row]') : null;
    if (!el) return;
    if (el.dataset.act) {
      const idx = items().findIndex((i) => i.act === el.dataset.act);
      if (idx >= 0) focus = idx;
      activate(el.dataset.act);
      return;
    }
    if (el.dataset.tab !== undefined) {
      tab = parseInt(el.dataset.tab, 10) || 0;
      focus = 0;
      sfx('ui');
      render();
      return;
    }
    if (el.dataset.row !== undefined && screen === 'settings') {
      const i = parseInt(el.dataset.row, 10);
      const row = TABS[tab].rows[i];
      focus = i;
      if (!row) return;
      const hit = e.target.closest('[data-v],[data-dir]');
      if (hit && hit.dataset.v !== undefined) {
        if (setRow(row, parseInt(hit.dataset.v, 10))) sfx('ui');
      } else if (hit && hit.dataset.dir !== undefined) {
        adjust(parseInt(hit.dataset.dir, 10));
        return;
      } else {
        press();
        return;
      }
      render();
    }
  }

  function onMove(e) {
    if (screen === null || !e.target || !e.target.closest) return;
    const el = e.target.closest('[data-act],[data-row]');
    if (!el) return;
    let idx = -1;
    if (el.dataset.row !== undefined && screen === 'settings') idx = parseInt(el.dataset.row, 10);
    else if (el.dataset.act) idx = items().findIndex((i) => i.act === el.dataset.act);
    if (idx >= 0 && idx !== focus) {
      focus = idx;
      // Only restyle the focus ring; a full re-render would eat the click that follows.
      const list = root().querySelectorAll('[data-act],[data-row]');
      list.forEach((n) => n.classList.remove('focus'));
      el.classList.add('focus');
    }
  }

  // ---------------------------------------------------------------- rendering
  const esc = (t) => String(t).replace(/[&<>"]/g, (ch) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[ch]));

  function motes() {
    let s = '';
    for (let i = 0; i < 16; i++) {
      const left = (i * 53 + 11) % 100;
      const delay = -((i * 37) % 19);
      const dur = 14 + ((i * 7) % 11);
      const size = 3 + ((i * 5) % 5);
      s += '<i class="mote" style="left:' + left + '%;animation-delay:' + delay + 's;animation-duration:' + dur + 's;width:' + size + 'px;height:' + size + 'px"></i>';
    }
    return s;
  }

  function listHtml() {
    return '<nav class="m-list">' + items().map((it, i) => {
      const dis = loading && (it.act === 'continue' || it.act === 'new' || it.act === 'erase');
      return '<button class="m-item' + (i === focus ? ' focus' : '') + (dis ? ' off' : '') + '" data-act="' + it.act + '">' +
        esc(it.label) + (it.note ? '<small>' + esc(it.note) + '</small>' : '') + '</button>';
    }).join('') + '</nav>';
  }

  function sliderHtml(row, v) {
    const [lo, hi] = rowBounds(row);
    let notches = '';
    for (let n = lo; n <= hi; n++) {
      // Slider notches: value n is shown as filled when the current value is >= n (volume 0 = all empty).
      const filled = n <= v && n > lo ? ' on' : '';
      if (n === lo) continue;
      notches += '<i class="notch' + filled + '" data-v="' + n + '"></i>';
    }
    const txt = row.show ? row.show(v) : String(v);
    return '<span class="ctl"><b class="arrow" data-dir="-1">‹</b><span class="notches">' + notches + '</span><b class="arrow" data-dir="1">›</b><span class="val">' + esc(txt) + '</span></span>';
  }

  function rowHtml(row, i) {
    const v = rowValue(row);
    let ctl = '';
    if (row.type === 'slider') ctl = sliderHtml(row, v);
    else if (row.type === 'toggle') {
      const on = v === 1;
      ctl = '<span class="ctl"><span class="pill' + (on ? ' on' : '') + '" data-v="' + (on ? 0 : 1) + '">' + esc(on ? (row.on || 'On') : (row.off || 'Off')) + '</span></span>';
    } else if (row.type === 'choice') {
      ctl = '<span class="ctl"><b class="arrow" data-dir="-1">‹</b><span class="val wide">' + esc(row.options[v]) + '</span><b class="arrow" data-dir="1">›</b></span>';
    } else if (row.type === 'button') {
      ctl = '<span class="ctl"><span class="pill btn">Reset</span></span>';
    }
    return '<div class="row' + (i === focus ? ' focus' : '') + '" data-row="' + i + '"><span class="lbl">' + esc(row.label) +
      (row.note ? '<small>' + esc(row.note) + '</small>' : '') + '</span>' + ctl + '</div>';
  }

  function settingsHtml() {
    const tabs = TABS.map((t, i) => '<button class="tab' + (i === tab ? ' on' : '') + '" data-tab="' + i + '">' + t.name + '</button>').join('');
    const cur = TABS[tab];
    let body;
    if (cur.controls) {
      body = '<div class="ctrls">' + CONTROLS.map((r) => '<div class="crow"><span>' + esc(r[0]) + '</span><span>' +
        r[1].split(' / ').map((k) => '<kbd>' + esc(k) + '</kbd>').join(' ') + '</span></div>').join('') +
        '<p class="soon">Rebinding keys is on the to-do list.</p></div>';
    } else {
      body = cur.rows.map(rowHtml).join('');
    }
    return '<div class="m-card wide"><h2>Settings</h2><div class="m-tabs">' + tabs + '</div><div class="m-rows">' + body + '</div>' +
      '<div class="m-foot"><span><kbd>↑</kbd><kbd>↓</kbd> choose · <kbd>←</kbd><kbd>→</kbd> change · <kbd>Tab</kbd> next section · <kbd>Esc</kbd> back</span>' +
      '<button class="m-back" data-act="back">Back</button></div></div>';
  }

  function render() {
    const el = root();
    if (!el) return;
    el.className = 'menu ' + (screen === null ? 'hidden' : 'show s-' + screen + (playing ? ' ingame' : ''));
    if (screen === null) { el.innerHTML = ''; return; }
    let html = '';
    if (screen === 'title') {
      html = '<div class="motes">' + motes() + '</div><div class="m-title"><div class="logo"><h1>' + esc(GAME_NAME) + '</h1><p>' + esc(TAGLINE) + '</p></div>' +
        (loading ? '<p class="loading">Lighting the lanterns…</p>' : '') + listHtml() +
        '<div class="m-foot hint"><kbd>↑</kbd><kbd>↓</kbd> choose · <kbd>Enter</kbd> select</div></div>' +
        '<div class="ver">v0.2 · work in progress</div>';
    } else if (screen === 'pause') {
      html = '<div class="m-card"><h2>Paused</h2>' + listHtml() + '<div class="m-foot hint"><kbd>Esc</kbd> resume</div></div>';
    } else if (screen === 'confirm') {
      html = '<div class="m-card"><h2>Start over?</h2><p class="warn">This erases your candles, upgrades and journal. It can\'t be undone.</p>' + listHtml() + '</div>';
    } else if (screen === 'settings') {
      html = settingsHtml();
    }
    el.innerHTML = html;
  }

  // ---------------------------------------------------------------- the bridge the Rust game uses
  window.gameMenu = {
    isOpen: () => screen !== null,
    // The title screen (or its settings): the world idles behind it. Pause just freezes the game.
    onTitle: () => screen !== null && !playing,
    takeAction: () => actions.shift() || '',
    settingsVersion: () => version,
    settingsText: () => serialize(settings),
    setHasSave: (has, summary) => { hasSave = !!has; saveSummary = summary ? String(summary) : ''; render(); },
    ready: () => { loading = false; render(); },
    openPause: () => { if (playing && screen === null) { sfx('ui'); setScreen('pause'); } },
    // for tests / debugging
    _state: () => ({ screen, playing, loading, hasSave, tab, focus, settings: Object.assign({}, settings) }),
  };

  window.addEventListener('keydown', onKey, true);
  document.addEventListener('click', onClick);
  document.addEventListener('mousemove', onMove);
  document.addEventListener('fullscreenchange', () => { if (screen === 'settings') render(); });

  function init() {
    applySettings();
    setScreen('title');
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
  // The audio engine is created by assets/audio.js; make sure it picks up the saved volumes once it exists.
  window.addEventListener('load', applySettings);
})();
