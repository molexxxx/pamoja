// The DocFX modern template loads this module from the template folder listed after it in
// docfx.json. It puts the site's bar over the C# reference by loading the script the other
// three references load; main.css beside it carries the palette and pulls the bar's styles.
export default {
  start()
  {
    const script = document.createElement('script');
    script.src = '/js/reference.js';
    script.defer = true;
    document.head.appendChild(script);
  },
};
