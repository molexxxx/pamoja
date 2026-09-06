// reference.js - the site's bar over the four generated API references.
//
// rustdoc, typedoc, pdoc, and DocFX each draw their own chrome, and a reader who follows a
// link into one of them leaves the site behind. This puts a slim bar above whatever the
// generator drew: the mark, the site's doors, and the way back to the reference page for
// the language the tree documents. Each generator loads it through its own hook, from the
// site root, so it works at any depth; reference.css makes room for it in each layout.

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

  const bar = document.createElement('header');
  bar.className = 'pamoja-bar';
  bar.innerHTML =
    `<a class="pamoja-bar-brand" href="/">${mark}<span>pamoja</span></a>`
    + '<nav aria-label="Site">'
    + '<a href="/docs/index.html">Docs</a>'
    + '<a href="/docs/hardware.html">Hardware</a>'
    + '<a href="/docs/reference/rust.html">Reference</a>'
    + '<a href="/dashboard/">Dashboard</a>'
    + '<a href="https://github.com/molexxxx/pamoja">GitHub</a>'
    + '</nav>'
    + (language ? `<a class="pamoja-bar-back" href="/docs/reference/${language}.html">${names[language]} reference: every package</a>` : '');
  document.body.prepend(bar);
  document.documentElement.classList.add('has-pamoja-bar');
})();
