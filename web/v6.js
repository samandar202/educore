'use strict';
/* EduCore V6 — 3D qatlam. Kutubxonasiz: canvas + perspektiv proyeksiya. */
(() => {
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const hover = matchMedia('(hover:hover)').matches;

  /* ---------- 3D sahna: aylanuvchi nuqtali globus + orbitalar + yo'ldoshlar ---------- */
  function scene(canvas) {
    const ctx = canvas.getContext('2d');
    const N = 520, pts = [];
    const ga = Math.PI * (3 - Math.sqrt(5)); // Fibonacci sfera
    for (let i = 0; i < N; i++) {
      const y = 1 - (i / (N - 1)) * 2, r = Math.sqrt(1 - y * y), t = ga * i;
      pts.push([Math.cos(t) * r, y, Math.sin(t) * r]);
    }
    const orbits = [
      { tilt: 1.15, spin: 0.35, r: 1.45, color: [139, 104, 212], sats: 3, speed: 0.45 },
      { tilt: -0.75, spin: 1.2, r: 1.7, color: [160, 190, 110], sats: 2, speed: -0.3 },
      { tilt: 0.25, spin: 2.3, r: 1.95, color: [229, 160, 120], sats: 2, speed: 0.22 },
    ];
    let w = 0, h = 0, dpr = 1, mx = 0, my = 0, tx = 0, ty = 0, t0 = performance.now(), visible = true, raf = 0;

    function size() {
      const b = canvas.getBoundingClientRect();
      dpr = Math.min(devicePixelRatio || 1, 2);
      w = b.width; h = b.height;
      canvas.width = Math.max(1, w * dpr); canvas.height = Math.max(1, h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }
    const rotY = (p, a) => [p[0] * Math.cos(a) + p[2] * Math.sin(a), p[1], -p[0] * Math.sin(a) + p[2] * Math.cos(a)];
    const rotX = (p, a) => [p[0], p[1] * Math.cos(a) - p[2] * Math.sin(a), p[1] * Math.sin(a) + p[2] * Math.cos(a)];
    const rotZ = (p, a) => [p[0] * Math.cos(a) - p[1] * Math.sin(a), p[0] * Math.sin(a) + p[1] * Math.cos(a), p[2]];

    function draw(now) {
      const t = (now - t0) / 1000;
      tx += (mx - tx) * 0.06; ty += (my - ty) * 0.06;
      ctx.clearRect(0, 0, w, h);
      const R = Math.min(w, h) * 0.2, cx = w / 2, cy = h / 2, fov = 3.2;
      const ay = t * 0.25 + tx * 0.6, ax = -0.35 + ty * 0.4;
      const proj = p => { const s = fov / (fov + p[2]); return [cx + p[0] * R * s, cy + p[1] * R * s, s, p[2]]; };
      const tf = p => rotX(rotY(p, ay), ax);

      // orqa orbitalar va yo'ldoshlar (z>0 orqada)
      const front = [];
      orbits.forEach((o, oi) => {
        const seg = 90;
        let prev = null;
        for (let i = 0; i <= seg; i++) {
          const a = (i / seg) * Math.PI * 2;
          let p = [Math.cos(a) * o.r, 0, Math.sin(a) * o.r];
          p = rotZ(rotX(p, o.tilt), o.spin);
          p = tf(p);
          const q = proj(p);
          if (prev) {
            const back = (p[2] + prev[3]) / 2 > 0;
            const alpha = back ? 0.12 : 0.38;
            const line = { a: prev, b: q, alpha, color: o.color };
            back ? strokeSeg(line) : front.push(() => strokeSeg(line));
          }
          prev = [q[0], q[1], q[2], p[2]];
        }
        for (let s = 0; s < o.sats; s++) {
          const a = t * o.speed + (s / o.sats) * Math.PI * 2 + oi;
          let p = [Math.cos(a) * o.r, 0, Math.sin(a) * o.r];
          p = tf(rotZ(rotX(p, o.tilt), o.spin));
          const q = proj(p);
          const fn = () => sat(q, o.color, p[2]);
          p[2] > 0 ? fn() : front.push(fn);
        }
      });

      // sfera nuqtalari
      const glow = ctx.createRadialGradient(cx - R * 0.3, cy - R * 0.35, R * 0.1, cx, cy, R * 1.25);
      glow.addColorStop(0, 'rgba(240,226,255,.95)');
      glow.addColorStop(0.55, 'rgba(176,140,232,.55)');
      glow.addColorStop(1, 'rgba(120,85,200,0)');
      ctx.fillStyle = glow;
      ctx.beginPath(); ctx.arc(cx, cy, R * 1.25, 0, Math.PI * 2); ctx.fill();
      for (const p0 of pts) {
        const p = tf(p0), q = proj(p);
        const depth = (1 - p[2]) / 2; // 0 orqa .. 1 old
        ctx.fillStyle = `rgba(${90 + depth * 40|0},${60 + depth * 30|0},${170 + depth * 50|0},${0.15 + depth * 0.75})`;
        ctx.beginPath(); ctx.arc(q[0], q[1], (0.6 + depth * 1.5) * q[2], 0, Math.PI * 2); ctx.fill();
      }
      front.forEach(f => f());
      if (!reduce && visible) raf = requestAnimationFrame(draw);
    }
    function strokeSeg(l) {
      ctx.strokeStyle = `rgba(${l.color[0]},${l.color[1]},${l.color[2]},${l.alpha})`;
      ctx.lineWidth = 1.2;
      ctx.beginPath(); ctx.moveTo(l.a[0], l.a[1]); ctx.lineTo(l.b[0], l.b[1]); ctx.stroke();
    }
    function sat(q, c, z) {
      const r = (z > 0 ? 4 : 7) * q[2];
      const g = ctx.createRadialGradient(q[0] - r * 0.4, q[1] - r * 0.4, r * 0.1, q[0], q[1], r);
      g.addColorStop(0, '#fff');
      g.addColorStop(1, `rgb(${c[0]},${c[1]},${c[2]})`);
      ctx.globalAlpha = z > 0 ? 0.45 : 1;
      ctx.shadowColor = `rgba(${c[0]},${c[1]},${c[2]},.6)`; ctx.shadowBlur = z > 0 ? 0 : 14;
      ctx.fillStyle = g; ctx.beginPath(); ctx.arc(q[0], q[1], r, 0, Math.PI * 2); ctx.fill();
      ctx.shadowBlur = 0; ctx.globalAlpha = 1;
    }

    size();
    new ResizeObserver(() => { size(); if (reduce) draw(performance.now()); }).observe(canvas);
    new IntersectionObserver(es => {
      visible = es[0].isIntersecting;
      cancelAnimationFrame(raf);
      if (visible) raf = requestAnimationFrame(draw);
    }).observe(canvas);
    if (hover) addEventListener('pointermove', e => {
      mx = e.clientX / innerWidth - 0.5; my = e.clientY / innerHeight - 0.5;
    }, { passive: true });
    raf = requestAnimationFrame(draw);
  }

  function upgradeScenes(root = document) {
    root.querySelectorAll('.clay-scene:not([data-v6])').forEach(s => {
      s.dataset.v6 = '1';
      const c = document.createElement('canvas');
      c.className = 'globe3d';
      c.setAttribute('aria-hidden', 'true');
      s.prepend(c);
      scene(c);
    });
  }

  /* ---------- Chuqurlik: kartalarni 3D qiyalanadigan qilish ---------- */
  const TILT = '.stat,.group-card,.feature-strip article,.assignment-card,.step,.evidence,.float-chip';
  function upgradeCards(root = document) {
    root.querySelectorAll(TILT).forEach(c => {
      if (c.dataset.v6) return;
      c.dataset.v6 = '1';
      c.classList.add('depth-card', 'v6-rise');
    });
    root.querySelectorAll('.panel:not([data-v6]),.studio-hero:not([data-v6]),.focus-strip:not([data-v6])').forEach(c => {
      c.dataset.v6 = '1'; c.classList.add('v6-rise');
    });
  }

  /* ---------- Kirish animatsiyasi (kaskad) ---------- */
  const io = 'IntersectionObserver' in window ? new IntersectionObserver(es => es.forEach(e => {
    if (e.isIntersecting) { e.target.classList.add('v6-in'); io.unobserve(e.target); }
  }), { threshold: 0.08 }) : null;
  function reveal() {
    let i = 0;
    document.querySelectorAll('.v6-rise:not(.v6-seen)').forEach(el => {
      el.classList.add('v6-seen');
      el.style.setProperty('--d', Math.min(i++, 10) * 45 + 'ms');
      if (reduce || !io) el.classList.add('v6-in'); else io.observe(el);
    });
  }

  /* ---------- Yorug'lik dog'i (kartada sichqoncha ortidan) ---------- */
  if (hover && !reduce) document.addEventListener('pointermove', e => {
    const c = e.target.closest && e.target.closest('.depth-card,.panel,.btn');
    if (!c) return;
    const r = c.getBoundingClientRect();
    c.style.setProperty('--lx', ((e.clientX - r.left) / r.width * 100) + '%');
    c.style.setProperty('--ly', ((e.clientY - r.top) / r.height * 100) + '%');
  }, { passive: true });

  function run() { upgradeScenes(); upgradeCards(); reveal(); }
  let pending = false;
  new MutationObserver(() => {
    if (pending) return; pending = true;
    requestAnimationFrame(() => { pending = false; run(); });
  }).observe(document.documentElement, { childList: true, subtree: true });
  document.readyState === 'loading' ? addEventListener('DOMContentLoaded', run) : run();
  document.documentElement.classList.add('v6');
})();
