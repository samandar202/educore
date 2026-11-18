'use strict';
/* EduCore V6 — hamjamiyat: erkin ro'yxat, tasdiqlash, farzand kodi, O'yin markazi. */
Object.assign(sections, {game: 'O‘yin markazi'});
Object.assign(iconPaths, {game: 'M6 8h12a4 4 0 0 1 4 4v3a3 3 0 0 1-5.4 1.8L15 15H9l-1.6 1.8A3 3 0 0 1 2 15v-3a4 4 0 0 1 4-4z M7 11v3 M5.5 12.5h3 M16 11.5h.01 M18 13.5h.01'});

let community = null;
async function getCommunity() {
  if (!community) { try { community = await api('/api/community'); } catch { community = {has_owner: false, signup: false}; } }
  return community;
}

/* ------------------------------ Kirish / ro'yxat ------------------------------ */
const roleCards = [
  ['student', 'O‘quvchi', 'Topshiriq, o‘yin va reyting', '✎', 'lilac'],
  ['teacher', 'O‘qituvchi', 'Rahbar tasdig‘idan so‘ng', '✦', 'mint'],
  ['parent', 'Ota-ona', 'Farzand kodi bilan', '♡', 'peach'],
];
const prevAuth = renderAuth;
renderAuth = async function (register) {
  const c = await getCommunity();
  if (!c.has_owner) { if (c.signup) { prevAuth(true); ownerHint(); } else prevAuth(false); return; }      // birinchi marta: egasi akademiyani ochadi
  prevAuth(false);
  const box = $('.auth-box'); if (!box) return;
  const toggle = box.querySelector('.auth-toggle');
  toggle.replaceChildren();
  for (const [isReg, label] of [[false, 'Hisobga kirish'], [true, 'Ro‘yxatdan o‘tish']]) {
    if (isReg && !c.signup) continue;
    const b = node('button', register === isReg ? 'selected' : '', label);
    b.type = 'button'; b.onclick = () => renderAuth(isReg); toggle.append(b);
  }
  box.querySelectorAll('.form-hint').forEach(n => n.remove());
  if (!register || !c.signup) {
    // Akademiya bitta: slug maydoni yashiriladi va avtomatik to'ldiriladi.
    const slug = box.querySelector('[name=slug]');
    if (slug) { slug.value = ''; slug.required = false; slug.closest('label').hidden = true; }
    const p = box.querySelector('p'); if (p) p.textContent = c.academy.name + ' kabinetiga email va parolingiz bilan kiring.';
    return;
  }
  // Ro'yxatdan o'tish formasi
  [...box.children].forEach(n => { if (n !== toggle) n.remove(); });
  let role = new URLSearchParams(location.search).get('role') || 'student';
  if (!roleCards.some(r => r[0] === role)) role = 'student';
  add(box, node('div', 'eyebrow', 'YANGI HISOB'), node('h1', '', 'Bizga qo‘shiling.'), node('p', '', c.academy.name + ' — o‘z rolingizni tanlang.'));
  const picker = node('div', 'role-picker');
  const form = node('form');
  const draw = () => {
    picker.replaceChildren();
    for (const [key, title, desc, glyph, color] of roleCards) {
      const b = node('button', 'role-card' + (role === key ? ' selected' : ''));
      b.type = 'button'; b.setAttribute('aria-pressed', role === key);
      add(b, node('span', 'mini-icon ' + color, glyph), node('b', '', title), node('small', '', desc));
      b.onclick = () => { role = key; draw(); };
      picker.append(b);
    }
    form.replaceChildren();
    field(form, 'Ism familiya', 'text', 'name', {required: true, minLength: 2, maxLength: 80, autocomplete: 'name'});
    field(form, 'Email', 'email', 'email', {required: true, maxLength: 160, autocomplete: 'username', placeholder: 'you@example.com'});
    field(form, 'Parol (kamida 10 belgi)', 'password', 'password', {required: true, minLength: 10, maxLength: 128, autocomplete: 'new-password'});
    if (role === 'parent') {
      const code = field(form, 'Farzandingiz kodi', 'text', 'child_code', {required: true, minLength: 6, maxLength: 6, placeholder: 'Masalan: K7M2QX', autocomplete: 'off'});
      code.style.textTransform = 'uppercase'; code.style.letterSpacing = '4px';
      form.append(node('p', 'form-hint', 'Kod farzandingiz kabinetidagi Bosh sahifada ko‘rinadi.'));
    }
    if (role === 'teacher') form.append(node('p', 'notice-box', 'O‘qituvchi hisobi akademiya rahbari tasdiqlagandan keyin ochiladi.'));
    formSubmit(form, role === 'teacher' ? 'So‘rov yuborish ↗' : 'Hisob yaratish ↗', async v => {
      const r = await api('/api/join', 'POST', {...v, role});
      if (r.pending) { pendingScreen(box); return; }
      section = 'overview'; history.replaceState(null, '', '/app'); await load();
      toast('Xush kelibsiz! Hisobingiz tayyor.');
    });
  };
  draw();
  add(box, picker, form);
};
function ownerHint() {
  const box = $('.auth-box'); if (!box) return;
  box.querySelector('.auth-toggle')?.remove();
  box.querySelectorAll('.form-hint').forEach(n => n.remove());
  box.append(node('p', 'form-hint', 'Bu birinchi va yagona akademiya egasi hisobi. U yaratilgach, boshqa hech kim akademiya ocha olmaydi.'));
}
function pendingScreen(box) {
  [...box.children].forEach(n => { if (!n.classList.contains('auth-toggle')) n.remove(); });
  add(box, add(node('div', 'pending-card'), node('div', 'pending-glyph', '⏳'), node('h1', '', 'So‘rovingiz yuborildi.'),
    node('p', '', 'Akademiya rahbari hisobingizni tasdiqlagach, shu email va parol bilan kira olasiz.'),
    btn('Kirish sahifasiga ↗', () => renderAuth(false), 'ghost')));
}

/* Landing: tugmalar matnini yangilash */
const prevLanding = landing;
landing = async function () {
  await prevLanding();
  const c = await getCommunity();
  if (!c.has_owner || !c.signup) return;
  document.querySelectorAll('a[href="/app?register=1"]').forEach(a => {
    const hero = !!a.closest('.hero-actions');
    a.firstChild.textContent = hero ? 'Hoziroq qo‘shilish ' : 'Ro‘yxatdan o‘tish';
  });
};

/* ------------------------------ Bosh sahifa qo'shimchalari ------------------------------ */
const prevOverview = overview;
overview = function (root) {
  prevOverview(root);
  const anchor = root.querySelector('.studio-hero, .hero-banner');
  const slot = node('div', 'v7-slot');
  anchor ? anchor.after(slot) : root.append(slot);
  if (owner()) pendingPanel(slot);
  if (data.user.role === 'student') studentStrip(slot);
  if (data.user.role === 'parent') parentStrip(slot);
};
async function pendingPanel(slot) {
  let list = [];
  try { list = await api('/api/pending'); } catch { return; }
  if (!list.length) return;
  const p = panel('Tasdiq kutayotgan o‘qituvchilar', list.length + ' TA SO‘ROV');
  p.classList.add('pending-panel');
  for (const t of list) {
    const row = add(node('div', 'pending-row'), node('div', 'avatar', initials(t.name)),
      add(node('div'), node('b', '', t.name), node('small', 'caption', t.email + ' · ' + date(t.created_at))));
    const act = node('div', 'button-row');
    add(act,
      btn('Tasdiqlash ✓', async () => { try { await api('/api/users/' + t.id + '/approve', 'POST'); toast(t.name + ' tasdiqlandi.'); await load(); } catch (e) { toast(e.message, true); } }, 'small'),
      btn('Rad etish', async () => { if (!confirm(t.name + ' so‘rovini rad etasizmi?')) return; try { await api('/api/users/' + t.id + '/reject', 'POST'); toast('So‘rov rad etildi.'); await load(); } catch (e) { toast(e.message, true); } }, 'danger small'));
    row.append(act); p.append(row);
  }
  slot.prepend(p);
}
async function studentStrip(slot) {
  const [code, g] = await Promise.all([api('/api/link-code').catch(() => null), api('/api/game').catch(() => null)]);
  const strip = node('div', 'v7-strip');
  if (g && g.me) strip.append(levelCard(g.me, true));
  if (code) {
    const c = add(node('section', 'panel code-card'), node('div', 'eyebrow', 'OTA-ONA UCHUN KOD'),
      node('div', 'code-value', code.code),
      node('p', 'form-hint', 'Ota-onangiz ro‘yxatdan o‘tishda shu kodni kiritadi va natijalaringizni ko‘radi.'));
    c.append(btn('Nusxalash', async () => { try { await navigator.clipboard.writeText(code.code); toast('Kod nusxalandi.'); } catch { toast('Kod: ' + code.code); } }, 'ghost small'));
    strip.append(c);
  }
  if (!data.user.group_id) strip.append(add(node('section', 'panel notice-panel'), node('b', '', 'Guruhga biriktirilmagansiz'),
    node('p', 'form-hint', 'Akademiya rahbari sizni guruhga qo‘shgach, topshiriqlar shu yerda paydo bo‘ladi. Hozircha O‘yin markazida mashq qiling!')));
  slot.append(strip);
}
async function parentStrip(slot) {
  const g = await api('/api/game').catch(() => null);
  const strip = node('div', 'v7-strip');
  (g?.children || []).forEach(k => strip.append(levelCard(k, false)));
  const add2 = add(node('section', 'panel code-card'), node('div', 'eyebrow', 'YANA FARZAND QO‘SHISH'));
  const f = node('form', 'inline-code');
  const inp = field(f, 'Farzand kodi', 'text', 'child_code', {required: true, minLength: 6, maxLength: 6, placeholder: 'K7M2QX'});
  inp.style.textTransform = 'uppercase';
  formSubmit(f, 'Bog‘lash', async v => { await api('/api/link-child', 'POST', v); toast('Farzandingiz bog‘landi.'); await load(); });
  add2.append(f); strip.append(add2);
  slot.append(strip);
}

/* ------------------------------ O'yin markazi ------------------------------ */
function levelCard(s, mine) {
  const pct = Math.round(100 * (s.xp - s.level_start) / Math.max(1, s.level_next - s.level_start));
  const card = node('section', 'panel level-card');
  const ring = node('div', 'level-ring'); ring.style.setProperty('--p', pct);
  add(ring, node('b', '', String(s.level)), node('small', '', 'DARAJA'));
  const info = node('div', 'level-info');
  add(info, node('div', 'eyebrow', mine ? 'MENING YUTUQLARIM' : 'FARZANDIM'), node('h3', '', s.name),
    node('div', 'xp-line', s.xp + ' XP · keyingi darajagacha ' + (s.level_next - s.xp) + ' XP'),
    add(node('div', 'progress-track xp-track'), (() => { const b = node('span'); b.style.width = pct + '%'; return b; })()),
    add(node('div', 'badge-mini'), ...s.badges.filter(b => b.earned).map(b => node('span', 'badge', '★ ' + b.title))));
  add(card, ring, info);
  if (section !== 'game') { card.classList.add('clickable'); card.onclick = () => go('game'); }
  return card;
}
function badgeGrid(s) {
  const g = node('div', 'badge-grid');
  const glyph = {first: '①', worker: '⚒', star: '★', attend: '✓', words: 'Aa', gamer: '♜'};
  for (const b of s.badges) g.append(add(node('div', 'badge-tile' + (b.earned ? ' earned' : '')),
    node('span', 'badge-gem', glyph[b.key] || '✦'), node('b', '', b.title), node('small', '', b.desc)));
  return g;
}
function boardPanel(board, title) {
  const p = panel(title, 'TOP ' + board.length);
  if (!board.length) { p.append(empty('Hali reyting yo‘q', 'O‘quvchilar topshiriq bajarib va o‘ynab XP yig‘adi.')); return p; }
  const list = node('ol', 'leaderboard');
  for (const r of board) list.append(add(node('li', 'lb-row' + (r.me ? ' me' : '') + (r.rank <= 3 ? ' top' + r.rank : '')),
    node('span', 'lb-rank', r.rank <= 3 ? ['🥇', '🥈', '🥉'][r.rank - 1] : String(r.rank)),
    node('span', 'avatar', initials(r.name)), add(node('div', 'lb-name'), node('b', '', r.name), node('small', 'caption', r.level + '-daraja')),
    node('strong', 'lb-xp', r.xp + ' XP')));
  p.append(list); return p;
}
function gamePage(root) {
  const student = data.user.role === 'student', parent = data.user.role === 'parent';
  heading(root, student ? 'O‘ynab o‘rgan. Yutib ko‘taril.' : parent ? 'Farzandingiz yutuqlari.' : 'O‘quvchilar reytingi.',
    student ? 'Topshiriq, davomat va o‘yinlar uchun XP oling, darajangizni oshiring.' : parent ? 'Daraja, nishonlar va guruhdagi o‘rni.' : 'XP: topshiriq, baho, davomat va so‘z o‘yini asosida hisoblanadi.');
  const box = node('div', 'game-wrap'); box.append(node('div', 'loading', 'Yuklanmoqda…')); root.append(box);
  api('/api/game').then(g => {
    box.replaceChildren();
    const layout = node('div', 'game-layout'), left = node('div'), right = node('div');
    if (student && g.me) { left.append(levelCard(g.me, true)); }
    if (parent) g.children.forEach(k => left.append(levelCard(k, false)));
    left.append(wordsPanel(g, student));
    const mine = student ? g.me : parent ? g.children[0] : null;
    if (mine) { const b = panel('Nishonlar', mine.badges.filter(x => x.earned).length + ' / ' + mine.badges.length); b.append(badgeGrid(mine)); left.append(b); }
    right.append(boardPanel(g.board, student || parent ? 'Guruh reytingi' : 'Akademiya reytingi'));
    const how = panel('XP qanday yig‘iladi?');
    how.append(add(node('ul', 'xp-rules'),
      node('li', '', 'Topshiriq yuborish — 10 XP'), node('li', '', 'Bahodan — har 5 ball uchun 1 XP'),
      node('li', '', 'Darsga kelish — 5 XP (kechiksa 3 XP)'), node('li', '', 'So‘z o‘yini — 1 to‘g‘ri javob = 1 XP (kuniga ' + g.daily_game_cap + ' XP gacha)')));
    right.append(how);
    add(layout, left, right); box.append(layout);
  }).catch(e => { box.replaceChildren(empty('Yuklab bo‘lmadi', e.message)); });
}

/* ------------------------------ So'z o'yini ------------------------------ */
const WORDS = [
  ['apple','olma'],['book','kitob'],['house','uy'],['water','suv'],['bread','non'],['school','maktab'],['teacher','o‘qituvchi'],['student','o‘quvchi'],
  ['friend','do‘st'],['family','oila'],['mother','ona'],['father','ota'],['brother','aka / uka'],['sister','opa / singil'],['child','bola'],['city','shahar'],
  ['village','qishloq'],['street','ko‘cha'],['car','mashina'],['window','deraza'],['door','eshik'],['table','stol'],['chair','stul'],['pen','ruchka'],
  ['pencil','qalam'],['bag','sumka'],['dog','it'],['cat','mushuk'],['horse','ot'],['bird','qush'],['fish','baliq'],['tree','daraxt'],
  ['flower','gul'],['sun','quyosh'],['moon','oy'],['star','yulduz'],['sky','osmon'],['rain','yomg‘ir'],['snow','qor'],['wind','shamol'],
  ['river','daryo'],['mountain','tog‘'],['sea','dengiz'],['road','yo‘l'],['bridge','ko‘prik'],['garden','bog‘'],['kitchen','oshxona'],['room','xona'],
  ['bed','karavot'],['clock','soat'],['money','pul'],['shop','do‘kon'],['market','bozor'],['hospital','kasalxona'],['doctor','shifokor'],['milk','sut'],
  ['meat','go‘sht'],['egg','tuxum'],['rice','guruch'],['sugar','shakar'],['salt','tuz'],['tea','choy'],['juice','sharbat'],['cheese','pishloq'],
  ['red','qizil'],['blue','ko‘k'],['green','yashil'],['yellow','sariq'],['white','oq'],['black','qora'],['big','katta'],['small','kichik'],
  ['new','yangi'],['old','eski'],['hot','issiq'],['cold','sovuq'],['happy','xursand'],['sad','xafa'],['fast','tez'],['slow','sekin'],
  ['beautiful','chiroyli'],['strong','kuchli'],['easy','oson'],['difficult','qiyin'],['early','erta'],['late','kech'],['today','bugun'],['tomorrow','ertaga'],
  ['yesterday','kecha'],['morning','ertalab'],['evening','kechqurun'],['night','tun'],['week','hafta'],['month','oy (vaqt)'],['year','yil'],['always','doim'],
  ['never','hech qachon'],['read','o‘qimoq'],['write','yozmoq'],['speak','gapirmoq'],['listen','tinglamoq'],['run','yugurmoq'],['walk','yurmoq'],['eat','yemoq'],
  ['drink','ichmoq'],['sleep','uxlamoq'],['open','ochmoq'],['close','yopmoq'],['buy','sotib olmoq'],['sell','sotmoq'],['learn','o‘rganmoq'],['help','yordam bermoq'],
  ['think','o‘ylamoq'],['know','bilmoq'],['want','xohlamoq'],['love','sevmoq'],['play','o‘ynamoq'],['work','ishlamoq'],['give','bermoq'],['take','olmoq'],
  ['head','bosh'],['hand','qo‘l'],['eye','ko‘z'],['heart','yurak'],['question','savol'],['answer','javob'],['word','so‘z'],['language','til'],
];
function shuffle(a) { for (let i = a.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; } return a; }
function wordsPanel(g, student) {
  const p = panel('So‘z o‘yini', '60 SONIYA');
  p.classList.add('words-panel');
  const best = student && g.me ? g.me.words_best : 0;
  const intro = add(node('div', 'words-intro'),
    add(node('div', 'words-cube'), node('span', 'cube-face f1', 'Aa'), node('span', 'cube-face f2', 'Bb'), node('span', 'cube-face f3', 'Cc'), node('span', 'cube-face f4', 'O‘')),
    add(node('div'), node('b', '', 'Inglizcha so‘zning tarjimasini toping'),
      node('p', 'form-hint', '60 soniyada imkon qadar ko‘p to‘g‘ri javob bering. Ketma-ket to‘g‘ri javoblar kombo beradi!' + (student ? ' Bugun o‘yindan: ' + g.today_game_xp + '/' + g.daily_game_cap + ' XP.' : ' (Xodimlar uchun sinov rejimi — XP yozilmaydi.)')),
      best ? node('div', 'badge', 'Rekordingiz: ' + best) : null,
      btn('▶ O‘yinni boshlash', () => playWords(p, student), 'dark')));
  p.append(intro);
  return p;
}
function playWords(p, student) {
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  let score = 0, combo = 0, left = 60, current = null, lock = false;
  const pool = shuffle(WORDS.slice());
  p.querySelector('.words-intro')?.remove(); p.querySelector('.words-stage')?.remove();
  const stage = node('div', 'words-stage');
  const hud = node('div', 'words-hud'), timer = node('div', 'words-timer'), scoreEl = node('b', '', '0'), comboEl = node('span', 'combo', '');
  add(hud, add(node('div'), node('small', 'caption', 'BALL'), scoreEl), comboEl, timer);
  const card = add(node('div', 'word-card'), node('small', 'caption', 'INGLIZCHA'), node('div', 'word-text', ''));
  const opts = node('div', 'word-options');
  add(stage, hud, add(node('div', 'word-scene'), card), opts);
  p.append(stage);
  const next = () => {
    if (!pool.length) pool.push(...shuffle(WORDS.slice()));
    current = pool.pop();
    const wrong = shuffle(WORDS.filter(w => w[1] !== current[1])).slice(0, 3).map(w => w[1]);
    card.querySelector('.word-text').textContent = current[0];
    card.classList.remove('flip'); void card.offsetWidth; if (!reduce) card.classList.add('flip');
    opts.replaceChildren();
    shuffle([current[1], ...wrong]).forEach(o => {
      const b = node('button', 'word-opt', o); b.type = 'button';
      b.onclick = () => {
        if (lock) return; lock = true;
        if (o === current[1]) { score++; combo++; b.classList.add('ok'); left = Math.min(60, left + (combo % 5 === 0 ? 3 : 0)); }
        else { combo = 0; b.classList.add('bad'); [...opts.children].find(x => x.textContent === current[1])?.classList.add('ok'); }
        scoreEl.textContent = score;
        comboEl.textContent = combo >= 2 ? '🔥 ×' + combo + (combo % 5 === 0 ? ' +3s' : '') : '';
        setTimeout(() => { lock = false; if (left > 0) next(); }, o === current[1] ? 280 : 700);
      };
      opts.append(b);
    });
  };
  const tick = () => {
    timer.textContent = left + 's'; timer.style.setProperty('--t', left / 60);
    if (left <= 0) return finish();
    left--; t = setTimeout(tick, 1000);
  };
  let t;
  const finish = async () => {
    clearTimeout(t);
    stage.replaceChildren();
    let xpText = student ? 'Natija saqlanmoqda…' : 'Sinov rejimi: XP yozilmadi.';
    const res = add(node('div', 'words-result'), node('div', 'result-score', String(score)), node('b', '', score >= 20 ? 'Ajoyib! Siz so‘z ustasisiz!' : score >= 10 ? 'Zo‘r natija!' : 'Yaxshi boshlanish!'), node('p', 'form-hint', xpText));
    stage.append(res);
    if (score >= 10 && !reduce) confetti(stage);
    if (student) {
      try {
        const r = await api('/api/game/score', 'POST', {game: 'words', score: Math.min(score, 200)});
        res.querySelector('.form-hint').textContent = r.xp > 0 ? '+' + r.xp + ' XP qo‘shildi!' + (r.capped ? ' (Bugungi o‘yin limiti to‘ldi.)' : '') : 'Bugungi o‘yin XP limiti to‘lgan. Ertaga yana!';
      } catch (e) { res.querySelector('.form-hint').textContent = e.message; }
    }
    res.append(add(node('div', 'button-row'), btn('↻ Yana o‘ynash', () => playWords(p, student), 'dark'), btn('Reytingni yangilash', () => go('game'), 'ghost')));
  };
  // sahifadan chiqilsa taymer to'xtaydi
  recorderCleanups.add(() => clearTimeout(t));
  next(); tick();
}
function confetti(host) {
  const c = node('div', 'confetti');
  const colors = ['#9b78e4', '#b8d27a', '#f0a77c', '#7cc4e8', '#f2d06b'];
  for (let i = 0; i < 40; i++) {
    const s = node('i');
    s.style.left = Math.random() * 100 + '%'; s.style.background = colors[i % colors.length];
    s.style.animationDelay = Math.random() * 0.4 + 's'; s.style.setProperty('--r', (Math.random() * 720 - 360) + 'deg');
    c.append(s);
  }
  host.append(c); setTimeout(() => c.remove(), 2600);
}
