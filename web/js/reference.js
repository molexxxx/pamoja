// reference.js - the site's bar over the four generated API references.
//
// rustdoc, typedoc, pdoc, and DocFX each draw their own chrome, and a reader who follows a
// link into one of them leaves the site behind. This puts a slim bar above whatever the
// generator drew: the mark, the site's doors, GitHub, and the way back to the reference
// page for the language the tree documents. Each generator loads it through its own hook,
// from the site root, so it works at any depth; reference.css makes room for it in each
// layout. The bar is built from div elements on purpose: pdoc styles every bare nav as its
// fixed sidebar, and the other generators have element rules of their own.

(() =>
{
  const match = location.pathname.match(/\/docs\/reference\/(rust|node|python|dotnet)\//);
  const language = match ? match[1] : null;
  const names = { rust: 'Rust', node: 'TypeScript', python: 'Python', dotnet: 'C#' };
  const mark =
    '<svg viewBox="0 0 240 240" width="22" height="22" aria-hidden="true">'
    + '<g fill="none" stroke-width="10"><polygon points="120,40 188,80 188,160 120,200 52,160 52,80" stroke="#FBF3E4" stroke-opacity="0.28"/>'
    + '<g stroke="#FBF3E4" stroke-opacity="0.22"><line x1="120" y1="120" x2="120" y2="40"/><line x1="120" y1="120" x2="188" y2="80"/><line x1="120" y1="120" x2="188" y2="160"/>'
    + '<line x1="120" y1="120" x2="120" y2="200"/><line x1="120" y1="120" x2="52" y2="160"/><line x1="120" y1="120" x2="52" y2="80"/></g></g>'
    + '<g><circle cx="120" cy="40" r="13" fill="#FFB627"/><circle cx="188" cy="80" r="13" fill="#F26A4B"/><circle cx="188" cy="160" r="13" fill="#1FA995"/>'
    + '<circle cx="120" cy="200" r="13" fill="#FFB627"/><circle cx="52" cy="160" r="13" fill="#F26A4B"/><circle cx="52" cy="80" r="13" fill="#1FA995"/></g>'
    + '<circle cx="120" cy="120" r="22" fill="#FFB627"/></svg>';
  const github =
    '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="currentColor">'
    + '<path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>';

  const bar = document.createElement('header');
  bar.className = 'pamoja-bar';
  bar.innerHTML =
    `<a class="pamoja-bar-brand" href="/">${mark}<span>pamoja</span></a>`
    + '<div class="pamoja-bar-nav" role="navigation" aria-label="Site">'
    + '<a href="/docs/index.html">Docs</a>'
    + '<a href="/docs/hardware.html">Hardware</a>'
    + '<a href="/docs/reference/rust.html">Reference</a>'
    + '</div>'
    + '<div class="pamoja-bar-end">'
    + (language ? `<a class="pamoja-bar-back" href="/docs/reference/${language}.html">${names[language]} reference: every package</a>` : '')
    + `<a class="pamoja-bar-icon" href="https://github.com/molexxxx/pamoja" title="GitHub" aria-label="GitHub">${github}</a>`
    + '</div>';
  document.body.prepend(bar);
  document.documentElement.classList.add('has-pamoja-bar');
})();
