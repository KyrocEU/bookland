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

// One-shot section reveals and a compositor-only reading line. No scroll work
// is scheduled while the page is still, and reduced-motion visitors see every
// section immediately.
const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)');
let stopScrollMotion = () => {};

function startScrollMotion() {
  if (motionPreference.matches || !('IntersectionObserver' in window)) return;
  const revealNodes = Array.from(document.querySelectorAll(
    '.hero-product, .section-intro, .demo, .capture-hero, .capture-card, .film-card, .feature, .flow-list > div, .source-layout > div, .faq-list details'
  ));
  for (const node of revealNodes) {
    node.classList.add('scroll-reveal');
    if (node.matches('.capture-card, .film-card, .feature')) {
      const index = Array.from(node.parentElement.children).indexOf(node);
      node.style.setProperty('--reveal-delay', `${Math.min(index % 3, 2) * 85}ms`);
    }
  }
  document.documentElement.classList.add('motion-enabled');
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      entry.target.classList.add('is-visible');
      observer.unobserve(entry.target);
    }
  }, { threshold: 0.08, rootMargin: '0px 0px -6% 0px' });
  revealNodes.forEach((node) => observer.observe(node));

  const progress = document.querySelector('.scroll-progress span');
  let scheduled = false;
  function updateProgress() {
    const distance = document.documentElement.scrollHeight - window.innerHeight;
    const fraction = distance > 0 ? Math.min(1, Math.max(0, window.scrollY / distance)) : 1;
    progress.style.transform = `scaleX(${fraction})`;
    scheduled = false;
  }
  function scheduleProgress() {
    if (scheduled) return;
    scheduled = true;
    window.requestAnimationFrame(updateProgress);
  }
  window.addEventListener('scroll', scheduleProgress, { passive: true });
  window.addEventListener('resize', scheduleProgress, { passive: true });
  scheduleProgress();

  stopScrollMotion = () => {
    observer.disconnect();
    window.removeEventListener('scroll', scheduleProgress);
    window.removeEventListener('resize', scheduleProgress);
    document.documentElement.classList.remove('motion-enabled');
    revealNodes.forEach((node) => node.classList.remove('scroll-reveal', 'is-visible'));
  };
}

startScrollMotion();
motionPreference.addEventListener('change', () => {
  stopScrollMotion();
  stopScrollMotion = () => {};
  startScrollMotion();
});
