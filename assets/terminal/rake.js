(() => {
  'use strict';

  const config = /*__RAKE_CONFIG__*/;
  const container = document.getElementById('terminal');
  const options = {
    cursorBlink: true,
    theme: config.theme
  };
  if (config.fontFamily) options.fontFamily = config.fontFamily;
  if (config.fontSize) options.fontSize = config.fontSize;
  const terminal = new Terminal(options);
  const fitAddon = new FitAddon.FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(container);

  // Hardware rendering for heavy output; if WebGL is unavailable or the
  // context is lost, dispose the addon and xterm keeps its DOM renderer.
  let webgl = null;
  try {
    // The UMD bundle exposes the namespace, so the class is one level down.
    webgl = new WebglAddon.WebglAddon();
    terminal.loadAddon(webgl);
    webgl.onContextLoss(() => webgl.dispose());
  } catch (error) {
    webgl = null;
  }

  const post = message => window.ipc.postMessage(JSON.stringify(message));
  let lastSize = '';
  const fit = () => {
    fitAddon.fit();
    const { cols, rows } = terminal;
    if (cols > 0 && rows > 0) {
      const size = `${cols}x${rows}`;
      if (size !== lastSize) {
        lastSize = size;
        post({ type: 'resize', cols, rows });
      }
    }
  };

  terminal.onData(data => post({ type: 'input', data }));
  terminal.attachCustomKeyEventHandler(event => {
    if (!event.metaKey || event.type !== 'keydown') return true;
    if (event.key.toLowerCase() === 'c' && terminal.hasSelection()) {
      post({ type: 'copy', data: terminal.getSelection() });
      return false;
    }
    if (event.key.toLowerCase() === 'v') {
      post({ type: 'paste' });
      return false;
    }
    return true;
  });

  window.rake = {
    writeBase64(encoded) {
      const binary = atob(encoded);
      const bytes = Uint8Array.from(binary, character => character.charCodeAt(0));
      terminal.write(bytes);
    },
    end(message) {
      terminal.write(`\r\n\r\n[${message}]\r\n`);
      terminal.options.disableStdin = true;
    },
    paste(text) {
      if (!terminal.options.disableStdin) terminal.paste(text);
    },
    focus() {
      terminal.focus();
    }
  };

  new ResizeObserver(fit).observe(container);
  fit();
  post({ type: 'ready' });
  terminal.focus();
})();
