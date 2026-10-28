"use strict";
(() => {
  const canvas = document.getElementById("avatar");
  const context = canvas.getContext("2d", {alpha: true});
  const label = document.getElementById("pose-label");
  const status = document.getElementById("status");
  const walk = document.getElementById("walk");
  const idle = document.getElementById("idle");
  const pause = document.getElementById("pause");
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");
  const image = new Image();
  const FRAME_W = 196;
  const FRAME_H = 300;
  const COLUMNS = 3;
  const CYCLE_MS = 800;
  const FRAMES = 8;
  const IDLE = 8;
  let motion = "walk";
  let paused = false;
  let epoch = null;
  let frozenElapsed = 0;
  let active = true;
  let raf = 0;

  function frameIndex(elapsed) {
    if (motion !== "walk" || reduce.matches) return IDLE;
    return Math.floor((elapsed % CYCLE_MS) / (CYCLE_MS / FRAMES));
  }
  function draw(index) {
    if (!image.complete || !image.naturalWidth || !context) return;
    context.clearRect(0, 0, FRAME_W, FRAME_H);
    context.imageSmoothingEnabled = true;
    context.drawImage(image, (index % COLUMNS) * FRAME_W, Math.floor(index / COLUMNS) * FRAME_H,
      FRAME_W, FRAME_H, 0, 0, FRAME_W, FRAME_H);
  }
  function updateControls() {
    const showingIdle = motion === "idle" || reduce.matches;
    walk.setAttribute("aria-pressed", String(motion === "walk"));
    idle.setAttribute("aria-pressed", String(motion === "idle"));
    pause.setAttribute("aria-pressed", String(paused));
    pause.textContent = paused ? "RESUME" : "PAUSE";
    label.textContent = reduce.matches ? "IDLE / REDUCED MOTION" : showingIdle ? "IDLE" : paused ? "WALK / PAUSED" : "WALK";
  }
  function tick(now) {
    raf = 0;
    if (!active) return;
    if (epoch === null) epoch = now;
    const elapsed = paused ? frozenElapsed : now - epoch;
    draw(frameIndex(elapsed));
    if (!paused && !reduce.matches && motion === "walk") raf = window.requestAnimationFrame(tick);
  }
  function restart() {
    if (raf) window.cancelAnimationFrame(raf);
    raf = 0;
    epoch = null;
    frozenElapsed = 0;
    updateControls();
    if (motion === "idle" || reduce.matches) draw(IDLE);
    else raf = window.requestAnimationFrame(tick);
  }
  walk.addEventListener("click", () => { motion = "walk"; paused = false; restart(); });
  idle.addEventListener("click", () => { motion = "idle"; paused = false; restart(); });
  pause.addEventListener("click", () => {
    // Pause freezes a genuine walk frame; reduced motion always selects idle.
    if (!paused) frozenElapsed = (performance.now() - (epoch ?? performance.now()));
    paused = !paused;
    if (!paused) epoch = performance.now() - frozenElapsed;
    updateControls();
    if (!paused && motion === "walk" && !reduce.matches) raf = window.requestAnimationFrame(tick);
  });
  reduce.addEventListener("change", restart);
  document.addEventListener("visibilitychange", () => {
    active = !document.hidden;
    if (active) restart();
  });
  image.addEventListener("load", () => { status.textContent = "APPROVED CHARACTER / LOCAL ASSET READY"; restart(); });
  image.addEventListener("error", () => { status.textContent = "ERROR / LOCAL ART ASSET UNAVAILABLE"; });
  image.src = "/assets/avatar-approved-atlas.png";
  updateControls();
})();
