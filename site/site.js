// Screen for me landing page
// 1. Media slots: elements with data-media / data-media-video name the real
//    asset they want. assets/media.json lists which assets actually exist;
//    listed assets replace the CSS recreations on load (see site/README.md).
// 2. Feature visuals play their little demo once when scrolled into view.

(function () {
  'use strict';

  // Resolve assets relative to this script's location, so the language
  // subpages (/es/, /fr/, ...) share the same assets/ folder as the root.
  const BASE = new URL('.', document.currentScript.src);

  // ---- media slot swapping -------------------------------------------------

  async function loadManifest() {
    try {
      const res = await fetch(new URL('assets/media.json', BASE));
      if (!res.ok) return [];
      const list = await res.json();
      return Array.isArray(list) ? list : [];
    } catch {
      return []; // file:// or malformed manifest: keep the recreations
    }
  }

  function hydrateSlot(slot, available) {
    const video = slot.dataset.mediaVideo;
    const image = slot.dataset.media;
    const alt = slot.dataset.mediaAlt || '';

    if (video && available.has(video)) {
      const el = document.createElement('video');
      el.src = new URL(video, BASE);
      el.controls = true;
      el.playsInline = true;
      el.preload = 'metadata';
      el.setAttribute('aria-label', alt);
      if (image && available.has(image)) el.poster = new URL(image, BASE);
      slot.replaceChildren(el);
      return;
    }

    if (image && available.has(image)) {
      const el = document.createElement('img');
      el.src = new URL(image, BASE);
      el.alt = alt;
      el.loading = 'lazy';
      el.decoding = 'async';
      slot.replaceChildren(el);
      slot.classList.add('has-media');
    }
  }

  loadManifest().then((list) => {
    if (!list.length) return;
    const available = new Set(list.map((name) => 'assets/' + String(name).replace(/^assets\//, '')));
    document
      .querySelectorAll('[data-media], [data-media-video]')
      .forEach((slot) => hydrateSlot(slot, available));
  });

  // ---- play-on-view for feature visuals -------------------------------------

  const visuals = document.querySelectorAll('.feature-visual .vis');
  if ('IntersectionObserver' in window && visuals.length) {
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            entry.target.classList.add('play');
            io.unobserve(entry.target);
          }
        }
      },
      { threshold: 0.45 }
    );
    visuals.forEach((v) => io.observe(v));
  } else {
    visuals.forEach((v) => v.classList.add('play'));
  }
})();
