// site.js - the behaviour behind the rendered documentation pages.
//
// Everything a page does without this file, it still does: the four language sections of
// a guide stack, the sidebar stays put, the code stays readable. This adds the language
// tabs and remembers the choice, the search box over search.json, the copy buttons, the
// sidebar drawer on a narrow screen, and the table of contents following the reader.
// Dependency-free and a few kilobytes, so a documentation page costs a reader on a slow
// link almost nothing beyond its text.

(() =>
{
  const root = document.documentElement;
  const base = root.dataset.root || '';
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (reducedMotion)
  {
    document.querySelectorAll('svg').forEach((svg) => { if (svg.pauseAnimations) svg.pauseAnimations(); });
  }

  /**
   * Escapes text for insertion as HTML.
   *
   * @param {string} text - the text.
   * @returns {string} the escaped text.
   */
  const esc = (text) => text.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

  // The sidebar keeps the current page in view, and is a drawer on a narrow screen.
  const toggle = document.querySelector('.menu-toggle');
  const side = document.getElementById('side');
  const current = side && side.querySelector('a.current');
  if (side && current && side.scrollHeight > side.clientHeight)
  {
    side.scrollTop = Math.max(0, current.offsetTop - side.clientHeight / 2);
  }
  if (toggle && side)
  {
    const setOpen = (open) =>
    {
      side.classList.toggle('open', open);
      toggle.setAttribute('aria-expanded', String(open));
    };
    toggle.addEventListener('click', () => setOpen(!side.classList.contains('open')));
    document.addEventListener('click', (e) =>
    {
      if (side.classList.contains('open') && !side.contains(e.target) && !toggle.contains(e.target)) setOpen(false);
    });
  }

  // Language tabs. The page head already chose the language (the hash, then the remembered
  // choice, then Rust) and the stylesheet shows that panel alone, so this only keeps the
  // tabs' state in step and answers clicks and hash changes.
  const LANG_KEY = 'pamoja:lang';
  const remember = (lang) => { try { localStorage.setItem(LANG_KEY, lang); } catch { /* private mode */ } };
  const tabs = [...document.querySelectorAll('.lang-tab')];
  if (tabs.length)
  {
    const known = new Set(tabs.map((tab) => tab.dataset.lang));
    const select = (lang) =>
    {
      root.dataset.lang = lang;
      tabs.forEach((tab) => tab.setAttribute('aria-selected', String(tab.dataset.lang === lang)));
    };
    select(known.has(root.dataset.lang) ? root.dataset.lang : 'rust');
    tabs.forEach((tab) => tab.addEventListener('click', () =>
    {
      select(tab.dataset.lang);
      remember(tab.dataset.lang);
      history.replaceState(null, '', '#' + tab.dataset.lang);
    }));
    addEventListener('hashchange', () =>
    {
      const lang = location.hash.slice(1);
      if (known.has(lang)) { select(lang); remember(lang); }
    });
  }

  // Copy buttons: an install line carries its text; a code figure copies its code.
  document.querySelectorAll('.copy').forEach((button) =>
  {
    button.addEventListener('click', async () =>
    {
      const figure = button.closest('figure');
      const text = button.dataset.copy ?? (figure ? figure.querySelector('code').textContent : '');
      try
      {
        await navigator.clipboard.writeText(text);
        button.textContent = 'copied';
        button.classList.add('done');
        setTimeout(() => { button.textContent = 'copy'; button.classList.remove('done'); }, 1400);
      } catch
      {
        button.textContent = 'select and copy';
      }
    });
  });

  // The table of contents follows the heading in view.
  const tocLinks = [...document.querySelectorAll('.toc a[href^="#"]')];
  if (tocLinks.length && 'IntersectionObserver' in window)
  {
    const byId = new Map(tocLinks.map((a) => [decodeURIComponent(a.hash.slice(1)), a]));
    const headings = [...byId.keys()].map((id) => document.getElementById(id)).filter(Boolean);
    const observer = new IntersectionObserver((entries) =>
    {
      entries.forEach((entry) =>
      {
        if (!entry.isIntersecting) return;
        tocLinks.forEach((a) => a.classList.remove('active'));
        byId.get(entry.target.id).classList.add('active');
      });
    }, { rootMargin: '-64px 0px -70% 0px' });
    headings.forEach((heading) => observer.observe(heading));
  }

  // Search: the index loads on first focus and is ranked here, heading matches first.
  const input = document.querySelector('.search-input');
  const results = document.querySelector('.search-results');
  if (input && results)
  {
    let index = null;
    let selected = -1;
    const load = async () =>
    {
      if (index) return index;
      try
      {
        const response = await fetch(base + 'search.json');
        index = await response.json();
      } catch
      {
        index = [];
      }
      return index;
    };
    const terms = (query) => query.toLowerCase().split(/\s+/).filter(Boolean);
    const score = (entry, words) =>
    {
      const title = entry.p.toLowerCase();
      const heading = entry.h.toLowerCase();
      const body = entry.b.toLowerCase();
      let total = 0;
      for (const word of words)
      {
        const inHeading = heading.includes(word);
        const inTitle = title.includes(word);
        const inBody = body.includes(word);
        if (!inHeading && !inTitle && !inBody) return 0;
        total += (inHeading ? 6 : 0) + (inTitle ? 4 : 0) + (inBody ? 1 : 0);
        if (heading === word || title === word) total += 3;
      }
      return entry.h ? total : total + 1;
    };
    const mark = (text, words) =>
    {
      let html = esc(text);
      for (const word of words)
      {
        const pattern = new RegExp(esc(word).replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'ig');
        html = html.replace(pattern, (m) => `<mark>${m}</mark>`);
      }
      return html;
    };
    const render = (hits, words) =>
    {
      selected = -1;
      results.hidden = false;
      if (!hits.length)
      {
        results.innerHTML = '<p class="search-empty">Nothing matches.</p>';
        return;
      }
      results.innerHTML = hits.map((entry, i) =>
        `<a class="result" role="option" id="result-${i}" href="${base}${entry.u}">` +
        `<span class="result-where">${esc(entry.s)}</span>` +
        `<span class="result-title">${mark(entry.p, words)}${entry.h ? ` <span class="result-sep">›</span> ${mark(entry.h, words)}` : ''}</span>` +
        `<span class="result-body">${mark(entry.b, words)}</span></a>`).join('');
    };
    input.addEventListener('focus', () => { load(); }, { once: true });
    input.addEventListener('input', async () =>
    {
      const words = terms(input.value);
      if (!words.length) { results.hidden = true; return; }
      const all = await load();
      const hits = all
        .map((entry) => ({ entry, s: score(entry, words) }))
        .filter((hit) => hit.s > 0)
        .sort((a, b) => b.s - a.s)
        .slice(0, 12)
        .map((hit) => hit.entry);
      render(hits, words);
    });
    input.addEventListener('keydown', (e) =>
    {
      const items = [...results.querySelectorAll('.result')];
      if (e.key === 'Escape')
      {
        results.hidden = true;
        input.blur();
      } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp')
      {
        if (!items.length) return;
        e.preventDefault();
        selected = (selected + (e.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
        items.forEach((item, i) => item.setAttribute('aria-selected', String(i === selected)));
        items[selected].scrollIntoView({ block: 'nearest' });
      } else if (e.key === 'Enter' && selected >= 0 && items[selected])
      {
        location.href = items[selected].href;
      }
    });
    document.addEventListener('click', (e) => { if (!e.target.closest('.search')) results.hidden = true; });
    addEventListener('keydown', (e) =>
    {
      if (e.key === '/' && !/^(input|textarea)$/i.test(document.activeElement.tagName))
      {
        e.preventDefault();
        input.focus();
      }
    });
  }
})();
