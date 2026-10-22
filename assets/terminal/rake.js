(() => {
  'use strict';

  const container = document.getElementById('terminal');
  const terminal = new Terminal({
    cursorBlink: true,
    fontFamily: 'ui-monospace, Menlo, Monaco, monospace',
    theme: { background: '#1e1e1e' }
  });
  const fitAddon = new FitAddon.FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(container);

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
