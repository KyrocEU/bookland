// Keep the site's own videos controllable from macOS media controls.
// Only the video the visitor actually played owns the session.
(() => {
  if (!('mediaSession' in navigator) || !('MediaMetadata' in window)) return;

  const videos = document.querySelectorAll('video');
  if (!videos.length) return;

  const session = navigator.mediaSession;
  let activeVideo = null;
  let lastPositionUpdate = -1;

  function updatePosition(force = false) {
    const video = activeVideo;
    if (!video || !session.setPositionState || !Number.isFinite(video.duration) || video.duration <= 0) return;
    const second = Math.floor(video.currentTime);
    if (!force && second === lastPositionUpdate) return;
    lastPositionUpdate = second;
    try {
      session.setPositionState({
        duration: video.duration,
        playbackRate: video.playbackRate,
        position: Math.min(video.duration, Math.max(0, video.currentTime))
      });
    } catch {
      // Some browsers expose Media Session without position support.
    }
  }

  function activate(video) {
    activeVideo = video;
    lastPositionUpdate = -1;
    const caption = video.closest('figure')?.querySelector('figcaption strong')?.textContent;
    session.metadata = new MediaMetadata({
      title: caption || 'Bookland · The island in action',
      artist: 'Bookland',
      artwork: [{ src: new URL('./bookland-icon.png', document.baseURI).href, sizes: '512x512', type: 'image/png' }]
    });
    session.playbackState = video.paused ? 'paused' : 'playing';
    updatePosition(true);
  }

  function seekTo(seconds) {
    const video = activeVideo;
    if (!video || !Number.isFinite(seconds)) return;
    const clamped = Math.max(0, seconds);
    video.currentTime = Number.isFinite(video.duration) ? Math.min(video.duration, clamped) : clamped;
    updatePosition(true);
  }

  const actions = {
    play: () => { activeVideo?.play()?.catch(() => {}); },
    pause: () => { activeVideo?.pause(); },
    seekto: ({ seekTime }) => seekTo(seekTime),
    seekbackward: ({ seekOffset }) => seekTo((activeVideo?.currentTime || 0) - (seekOffset || 15)),
    seekforward: ({ seekOffset }) => seekTo((activeVideo?.currentTime || 0) + (seekOffset || 15))
  };
  for (const [action, handler] of Object.entries(actions)) {
    try { session.setActionHandler(action, handler); } catch { /* Unsupported action. */ }
  }

  for (const video of videos) {
    video.addEventListener('play', () => activate(video));
    video.addEventListener('pause', () => {
      if (activeVideo !== video) return;
      session.playbackState = 'paused';
      updatePosition(true);
    });
    video.addEventListener('timeupdate', () => {
      if (activeVideo === video) updatePosition();
    });
    video.addEventListener('seeked', () => {
      if (activeVideo === video) updatePosition(true);
    });
    video.addEventListener('ended', () => {
      if (activeVideo === video) session.playbackState = 'none';
    });
  }
})();
