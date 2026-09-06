(() =>
{
  const root = document.documentElement;
  const base = root.dataset.root || '/';
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const LANG_KEY = 'pamoja:lang';

  /**
   * Escapes text for insertion as HTML.
   *
   * @param {string} text - the text.
   * @returns {string} the escaped text.
   */
  const esc = (text) => text.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

  const remember = (lang) => { try { localStorage.setItem(LANG_KEY, lang); } catch { /* private mode */ } };

  let tocObserver = null;

  /**
   * Binds the per-page behaviour inside `scope`: everything that reads the page rather than
   * the header, so it runs again after the page is swapped.
   *
   * @param {ParentNode} scope - the element holding the page, `document` at load.
   * @returns {void}
   */
  const bind = (scope) =>
  {
    if (reducedMotion)
    {
      scope.querySelectorAll('svg').forEach((svg) => { if (svg.pauseAnimations) svg.pauseAnimations(); });
    }

    // The sidebar keeps the current page in view.
    const side = document.getElementById('side');
    const current = side && side.querySelector('a.current');
    if (side && current && side.scrollHeight > side.clientHeight)
    {
      side.scrollTop = Math.max(0, current.offsetTop - side.clientHeight / 2);
    }

    // Language tabs. The page head chose the language (the hash, then the remembered
    // choice, then Rust) and the stylesheet shows that panel alone, so this only keeps the
    // tabs' state in step and answers clicks.
    const tabs = [...scope.querySelectorAll('.lang-tab')];
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
        history.replaceState(history.state, '', '#' + tab.dataset.lang);
      }));
    }

    // Copy buttons: an install line carries its text; a code figure copies its code.
    scope.querySelectorAll('.copy').forEach((button) =>
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
    if (tocObserver) tocObserver.disconnect();
    const tocLinks = [...scope.querySelectorAll('.toc a[href^="#"]')];
    if (tocLinks.length && 'IntersectionObserver' in window)
    {
      const byId = new Map(tocLinks.map((a) => [decodeURIComponent(a.hash.slice(1)), a]));
      const headings = [...byId.keys()].map((id) => document.getElementById(id)).filter(Boolean);
      tocObserver = new IntersectionObserver((entries) =>
      {
        entries.forEach((entry) =>
        {
          if (!entry.isIntersecting) return;
          tocLinks.forEach((a) => a.classList.remove('active'));
          byId.get(entry.target.id).classList.add('active');
        });
      }, { rootMargin: '-64px 0px -70% 0px' });
      headings.forEach((heading) => tocObserver.observe(heading));
    }
  };

  // A hash change selects the tab it names.
  addEventListener('hashchange', () =>
  {
    const lang = location.hash.slice(1);
    if (/^(rust|typescript|python|c)$/.test(lang))
    {
      root.dataset.lang = lang;
      remember(lang);
      document.querySelectorAll('.lang-tab').forEach((tab) => tab.setAttribute('aria-selected', String(tab.dataset.lang === lang)));
    }
  });

  // The sidebar is a drawer on a narrow screen. The button lives in the header and the
  // sidebar in the page, so the sidebar is looked up on each click.
  const toggle = document.querySelector('.menu-toggle');
  const syncToggle = () => { if (toggle) toggle.hidden = !document.getElementById('side'); };
  syncToggle();
  if (toggle)
  {
    const setOpen = (open) =>
    {
      const side = document.getElementById('side');
      if (!side) return;
      side.classList.toggle('open', open);
      toggle.setAttribute('aria-expanded', String(open));
    };
    toggle.addEventListener('click', () =>
    {
      const side = document.getElementById('side');
      setOpen(!(side && side.classList.contains('open')));
    });
    document.addEventListener('click', (e) =>
    {
      const side = document.getElementById('side');
      if (side && side.classList.contains('open') && !side.contains(e.target) && !toggle.contains(e.target)) setOpen(false);
    });
  }

  // Navigation without a reload. A link to another page of this site is fetched, the part
  // between the header and the footer is swapped for the fetched page's, the title, the
  // description, and the card follow, and the address is pushed. Anything that is not one
  // of this site's pages (another origin, a file, a generated reference tree, a link that
  // opens elsewhere) is left to the browser, as is any fetch that fails.
  const progress = document.createElement('div');
  progress.className = 'nav-progress';
  progress.setAttribute('aria-hidden', 'true');
  document.body.appendChild(progress);
  const cache = new Map();
  let inflight = null;
  history.scrollRestoration = 'manual';

  const isPage = (url) =>
    url.origin === location.origin
    && (/\.html$/.test(url.pathname) || url.pathname.endsWith('/'))
    && !/^\/docs\/reference\/(rust|node|python|dotnet)\//.test(url.pathname)
    && url.pathname !== '/404.html';

  const fetchPage = (href) =>
  {
    if (!cache.has(href))
    {
      cache.set(href, fetch(href, { headers: { 'X-Requested-With': 'pamoja' } })
        .then((response) => { if (!response.ok) throw new Error(String(response.status)); return response.text(); })
        .catch((err) => { cache.delete(href); throw err; }));
    }
    return cache.get(href);
  };

  const syncHead = (doc) =>
  {
    document.title = doc.title;
    for (const selector of ['meta[name="description"]', 'link[rel="canonical"]', 'meta[property="og:title"]', 'meta[property="og:description"]', 'meta[property="og:url"]', 'meta[property="og:type"]'])
    {
      const fresh = doc.head.querySelector(selector);
      const mine = document.head.querySelector(selector);
      if (fresh && mine) mine.replaceWith(document.adoptNode(fresh));
    }
    doc.head.querySelectorAll('link[rel="stylesheet"]').forEach((link) =>
    {
      const href = link.getAttribute('href');
      const path = href.split('?')[0];
      const mine = [...document.head.querySelectorAll('link[rel="stylesheet"]')].find((l) => l.getAttribute('href').split('?')[0] === path);
      if (!mine) document.head.appendChild(document.adoptNode(link));
      else if (mine.getAttribute('href') !== href) mine.replaceWith(document.adoptNode(link));
    });
  };

  const swapTo = (doc, url, scrollY) =>
  {
    const next = doc.getElementById('page');
    const page = document.getElementById('page');
    if (!next || !page) return false;
    syncHead(doc);
    document.body.className = doc.body.className;
    page.replaceWith(document.adoptNode(next));
    bind(next);
    syncToggle();
    if (next.querySelector('.home'))
    {
      const stamp = doc.documentElement.dataset.stamp || root.dataset.stamp || '';
      import(`/js/home.js?v=${stamp}`).then((home) => home.init()).catch(() => {});
    }
    const target = url.hash ? document.getElementById(decodeURIComponent(url.hash.slice(1))) : null;
    if (target) target.scrollIntoView();
    else window.scrollTo(0, scrollY);
    return true;
  };

  const navigate = async (href, { push = true, scrollY = 0 } = {}) =>
  {
    const url = new URL(href, location.href);
    inflight = url.href;
    progress.classList.add('on');
    let html;
    try { html = await fetchPage(url.href); }
    catch { location.href = url.href; return; }
    if (inflight !== url.href) return;
    const doc = new DOMParser().parseFromString(html, 'text/html');
    if (push)
    {
      history.replaceState({ scrollY: window.scrollY }, '', location.href);
      history.pushState({ scrollY: 0 }, '', url.href);
    }
    const lang = url.hash.slice(1);
    if (/^(rust|typescript|python|c)$/.test(lang)) root.dataset.lang = lang;
    const swap = () => { if (!swapTo(doc, url, scrollY)) location.href = url.href; };
    if (document.startViewTransition && !reducedMotion)
    {
      await document.startViewTransition(swap).finished.catch(() => {});
    } else
    {
      swap();
    }
    // A newer navigation may have started during the fade; only the latest clears the marks.
    if (inflight === url.href)
    {
      progress.classList.remove('on');
      inflight = null;
    }
  };

  const lightbox = (a) =>
  {
    const source = a.querySelector('picture source');
    const img = a.querySelector('img');
    const src = source && matchMedia(source.media).matches ? source.srcset : a.href;
    const view = document.createElement('div');
    view.className = 'lightbox-view';
    view.setAttribute('role', 'dialog');
    view.setAttribute('aria-modal', 'true');
    view.setAttribute('aria-label', img ? img.alt : 'Drawing');
    const full = new Image();
    full.src = src;
    full.alt = img ? img.alt : '';
    const close = document.createElement('button');
    close.type = 'button';
    close.className = 'lightbox-close';
    close.textContent = 'Close';
    close.setAttribute('aria-label', 'Close the drawing');
    const shut = () =>
    {
      view.remove();
      document.body.classList.remove('lightbox-open');
      removeEventListener('keydown', onKey);
      a.focus();
    };
    const onKey = (e) => { if (e.key === 'Escape') shut(); };
    view.addEventListener('click', (e) => { if (e.target === view || e.target === close) shut(); });
    addEventListener('keydown', onKey);
    view.append(full, close);
    document.body.append(view);
    document.body.classList.add('lightbox-open');
    close.focus();
  };

  document.addEventListener('click', (e) =>
  {
    const a = e.target.closest('a.lightbox');
    if (!a || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    e.preventDefault();
    lightbox(a);
  });

  document.addEventListener('click', (e) =>
  {
    const a = e.target.closest('a[href]');
    if (!a || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    if ((a.target && a.target !== '_self') || a.hasAttribute('download')) return;
    const url = new URL(a.href, location.href);
    if (!isPage(url)) return;
    if (url.pathname === location.pathname && url.search === location.search)
    {
      if (url.hash) return;
      e.preventDefault();
      window.scrollTo(0, 0);
      return;
    }
    e.preventDefault();
    navigate(url.href);
  });

  document.addEventListener('mouseover', (e) =>
  {
    const a = e.target.closest('a[href]');
    if (!a) return;
    const url = new URL(a.href, location.href);
    if (isPage(url) && url.pathname !== location.pathname) fetchPage(url.href).catch(() => {});
  });

  addEventListener('popstate', (e) =>
  {
    navigate(location.href, { push: false, scrollY: (e.state && e.state.scrollY) || 0 });
  });

  // Search: the index loads on first focus and is ranked here, heading matches first. The
  // header is never swapped, so this binds once.
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
        results.hidden = true;
        navigate(items[selected].href);
      }
    });
    document.addEventListener('click', (e) => { if (!e.target.closest('.search')) results.hidden = true; });
    results.addEventListener('click', (e) => { if (e.target.closest('.result')) results.hidden = true; });
    addEventListener('keydown', (e) =>
    {
      const typing = /^(input|textarea|select)$/i.test(document.activeElement.tagName) || document.activeElement.isContentEditable;
      if (e.key === '/' && !typing && !e.ctrlKey && !e.metaKey && !e.altKey)
      {
        e.preventDefault();
        input.focus();
        input.select();
      }
    });
  }

  bind(document);
})();
