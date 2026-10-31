// Shows the page in one language; the button swaps it.
const asked = new URLSearchParams(location.search).get("lang");
let lang = asked === "en" || asked === "zh" ? asked : navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
const button = document.querySelector("nav button");
const show = () => {
  for (const article of document.querySelectorAll("article")) {
    article.hidden = (article.lang === "zh-CN") !== (lang === "zh");
    if (!article.hidden) document.title = article.dataset.title;
  }
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  button.textContent = lang === "zh" ? "English" : "中文";
  button.lang = lang === "zh" ? "en" : "zh-CN";
};
button.addEventListener("click", () => {
  lang = lang === "zh" ? "en" : "zh";
  history.replaceState(null, "", `?lang=${lang}${location.hash}`);
  show();
});
show();
