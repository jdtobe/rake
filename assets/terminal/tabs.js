window.createTabStrip = (post, config) => {
  'use strict';
  if (!config.workspace) return null;
  const root = document.getElementById('tabstrip');
  root.hidden = false;
  document.body.dataset.tabPosition = config.tab_position || 'bottom';
  document.body.dataset.appearance = config.appearance || 'system';
  const list = document.createElement('div');
  list.className = 'tab-list'; list.setAttribute('role','tablist'); list.setAttribute('aria-label','Terminal sessions');
  const plus = document.createElement('button');
  plus.className = 'new-tab'; plus.textContent = '+'; plus.setAttribute('aria-label','New tab'); plus.title = 'New tab (⌘T)';
  plus.onclick = () => post({type:'new_tab'});
  root.append(list, plus);
  let tabs = [], active = null, editor = null;
  function render() {
    list.replaceChildren();
    for (const tab of tabs) {
      const item = document.createElement('div'); item.className = 'tab-item';
      const button = document.createElement('button');
      button.className = 'tab'; button.setAttribute('role','tab'); button.setAttribute('aria-selected',tab.id === active);
      button.setAttribute('aria-label',tab.title); button.title = tab.title;
      if (tab.busy) {const dot=document.createElement('span');dot.className='busy';dot.setAttribute('aria-hidden','true');button.append(dot);}
      const name = document.createElement('span');name.className='tab-name';name.textContent=tab.title;button.append(name);
      button.onclick = () => post({type:'select_tab',id:tab.id});
      button.ondblclick = () => {
        if (editor) return;
        const input = document.createElement('input');input.className='tab-name-editor';input.setAttribute('aria-label','Tab name');
        input.value=tab.manual || '';input.placeholder='Automatic name';input.maxLength=512;
        editor={id:tab.id,input};name.replaceWith(input);
        const finish = save => {
          if (!editor) return;
          const text=input.value;editor=null;render();
          if (save) post({type:'rename_tab',id:tab.id,text});
        };
        input.onclick=event=>event.stopPropagation();input.ondblclick=event=>event.stopPropagation();
        input.onkeydown=event=>{
          event.stopPropagation();
          if(event.key==='Enter'){event.preventDefault();finish(true);}
          else if(event.key==='Escape'){event.preventDefault();finish(false);}
        };
        input.onblur=()=>finish(false);
        input.focus();input.select();
      };
      const close=document.createElement('button');close.className='close-tab';close.textContent='×';close.setAttribute('aria-label',`Close ${tab.title}`);
      close.onclick=()=>post({type:'close_tab',id:tab.id});
      item.classList.toggle('active',tab.id===active);item.append(button,close);list.append(item);
      if(tab.id===active) requestAnimationFrame(()=>item.scrollIntoView({block:'nearest',inline:'nearest'}));
    }
  }
  document.addEventListener('keydown',event=>{
    if(event.target.closest?.('input, [contenteditable="true"]') && !event.target.classList.contains('xterm-helper-textarea')) return;
    let message=null;
    if(event.metaKey && !event.ctrlKey && !event.altKey && /^[1-9]$/.test(event.key)) message={type:'select_number',number:Number(event.key)};
    else if(event.ctrlKey && !event.metaKey && !event.altKey && event.key==='Tab') message={type:'cycle_tab',backwards:event.shiftKey};
    if(message){event.preventDefault();event.stopImmediatePropagation();if(!event.repeat)post(message);}
  },true);
  return {update(next,selected){tabs=next;active=selected;if(editor&&tabs.some(t=>t.id===editor.id))return;editor=null;render();}};
};
