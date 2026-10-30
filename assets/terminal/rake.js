(() => {
  'use strict';
  const config = /*__RAKE_CONFIG__*/;
  const container = document.getElementById('terminal');
  const sessions = new Map();
  let active = null;
  const legacy = !config.workspace;
  const post = message => {
    if (legacy && message.type === 'output_ack') return;
    if (legacy) { message = {...message}; delete message.id; }
    window.ipc.postMessage(JSON.stringify(message));
  };
  const strip = window.createTabStrip?.(post, config);

  function fit(session) {
    if (!session || !session.container.clientWidth || !session.container.clientHeight) return;
    session.fitAddon.fit();
    const {cols, rows} = session.terminal;
    const size = `${cols}x${rows}`;
    if (cols > 0 && rows > 0 && session.lastSize !== size) {
      session.lastSize = size;
      post({type:'resize', id:session.id, cols, rows});
    }
  }
  function releaseRenderer(session) {
    const addon = session?.webgl;
    if (session) session.webgl = null;
    if (addon) addon.dispose();
  }
  function select(id) {
    const next = sessions.get(id);
    if (!next) return;
    if (active !== id) {
      releaseRenderer(sessions.get(active));
      for (const session of sessions.values()) session.container.hidden = session.id !== id;
      active = id;
    }
    if (!next.webgl) {
      try {
        const webgl = new WebglAddon.WebglAddon();
        next.terminal.loadAddon(webgl);
        next.webgl = webgl;
        webgl.onContextLoss(() => releaseRenderer(next));
      } catch (_) { releaseRenderer(next); }
    }
    fit(next);
    next.terminal.refresh(0, next.terminal.rows - 1);
    next.terminal.focus();
  }
  function create(id) {
    if (sessions.has(id)) return;
    const node = document.createElement('div');
    node.className = 'terminal-session';
    node.dataset.terminalId = String(id);
    node.hidden = true;
    container.append(node);
    const options = {cursorBlink:true, theme:config.theme, screenReaderMode:true};
    if (config.fontFamily) options.fontFamily = config.fontFamily;
    if (config.fontSize) options.fontSize = config.fontSize;
    const terminal = new Terminal(options);
    const fitAddon = new FitAddon.FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(node);
    const session = {id, container:node, terminal, fitAddon, webgl:null, lastSize:''};
    sessions.set(id, session);
    terminal.onData(data => post({type:'input', id, data}));
    terminal.onTitleChange(text => {if (sessions.has(id)) post({type:'title', id, text});});
    terminal.attachCustomKeyEventHandler(event => {
      if (!event.metaKey || event.type !== 'keydown') return true;
      if (event.key.toLowerCase() === 'c' && terminal.hasSelection()) {
        post({type:'copy', id, data:terminal.getSelection()}); return false;
      }
      if (event.key.toLowerCase() === 'v') {post({type:'paste', id}); return false;}
      return true;
    });
  }
  function apply(message) {
    switch (message.type) {
      case 'create_tab': create(message.id); break;
      case 'select_tab': select(message.id); break;
      case 'remove_tab': {
        const session = sessions.get(message.id);
        if (!session) break;
        sessions.delete(message.id);
        releaseRenderer(session);
        session.terminal.dispose(); session.container.remove();
        if (active === message.id) active = null;
        break;
      }
      case 'tab_metadata': strip?.update(message.tabs, message.active); break;
      case 'notice': {
        const notice = document.getElementById('notice');
        notice.textContent = message.text; notice.hidden = false; break;
      }
      case 'paste': {
        const session = sessions.get(message.id);
        if (session && !session.terminal.options.disableStdin) session.terminal.paste(message.text);
        break;
      }
      case 'output': case 'end': {
        const session = sessions.get(message.id);
        if (!session) break;
        let data;
        if (message.type === 'output') {
          const binary = atob(message.data);
          data = Uint8Array.from(binary, character => character.charCodeAt(0));
        } else {
          data = `\r\n\r\n[${message.message}]\r\n`;
          session.terminal.options.disableStdin = true;
        }
        session.terminal.write(data, () => {
          if (sessions.get(message.id) === session) post({type:'output_ack', id:message.id, sequence:message.sequence});
        });
        break;
      }
    }
  }
  window.rake = {
    apply,
    focus(id = active) { if (sessions.has(id)) sessions.get(id).terminal.focus(); },
    // Removed with the single-session Rust wiring at workspace integration.
    writeBase64(data) {apply({type:'output',id:1,sequence:0,data});},
    end(message) {apply({type:'end',id:1,sequence:0,message});},
    paste(text) {apply({type:'paste',id:1,text});}
  };
  new ResizeObserver(() => fit(sessions.get(active))).observe(container);
  if (legacy) {create(1);select(1);}
  post({type:'ready'});
})();
