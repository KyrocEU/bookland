const tabs = Array.from(document.querySelectorAll('[data-tab]'));
const panes = Array.from(document.querySelectorAll('[data-pane]'));
const panel = document.getElementById('demo-panel');

function selectWorkspace(name, focus = false) {
  const selected = tabs.find((tab) => tab.dataset.tab === name);
  if (!selected) return;
  for (const tab of tabs) {
    const active = tab === selected;
    tab.classList.toggle('is-active', active);
    tab.setAttribute('aria-selected', String(active));
    tab.tabIndex = active ? 0 : -1;
  }
  for (const pane of panes) {
    const active = pane.dataset.pane === name;
    pane.hidden = !active;
    pane.classList.toggle('is-active', active);
  }
  panel.setAttribute('aria-labelledby', selected.id);
  if (focus) selected.focus();
}

for (const [index, tab] of tabs.entries()) {
  tab.addEventListener('click', () => selectWorkspace(tab.dataset.tab));
  tab.addEventListener('keydown', (event) => {
    let next;
    if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
    if (event.key === 'ArrowLeft') next = (index - 1 + tabs.length) % tabs.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = tabs.length - 1;
    if (next === undefined) return;
    event.preventDefault();
    selectWorkspace(tabs[next].dataset.tab, true);
  });
}

const copyButton = document.querySelector('.copy-command');
copyButton?.addEventListener('click', async () => {
  const commands = [
    'git clone https://github.com/KyrocEU/bookland.git',
    'cd bookland',
    'cargo test',
    'sh package.sh',
    'open Bookland.app'
  ].join('\n');
  try {
    await navigator.clipboard.writeText(commands);
    copyButton.textContent = 'Copied';
  } catch {
    copyButton.textContent = 'Select commands above';
  }
  window.setTimeout(() => { copyButton.textContent = 'Copy commands'; }, 2500);
});
