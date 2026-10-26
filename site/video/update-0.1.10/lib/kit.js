/* Ely 0.1.10 film: shared UI kit (icons, sample mail, the ElyMail window).
   Usage inside a scene: ElyKit.icon("inbox") -> "<svg ...>" string,
   ElyKit.mailWindow(container, { id: "s3", select: 0, box: "all" }) -> refs. */
(function () {
  "use strict";
  var P = {
    inbox: '<path d="M3 13h5l1.5 3h5L16 13h5"/><path d="M5.5 5h13L21 13v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-5z"/>',
    send: '<path d="M21 3 10 14"/><path d="M21 3l-7 18-4-7-7-4z"/>',
    draft: '<path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/><path d="M8 14h8M8 17.5h5"/>',
    outbox: '<path d="M12 15V3"/><path d="m7 8 5-5 5 5"/><path d="M3 14v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4"/>',
    archive: '<rect x="3" y="4" width="18" height="5" rx="1.5"/><path d="M5 9v9a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V9"/><path d="M10 13h4"/>',
    trash: '<path d="M4 7h16"/><path d="M9 7V4h6v3"/><path d="M6 7l1 13h10l1-13"/>',
    star: '<path d="m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"/>',
    clip: '<path d="m21 11-8.6 8.6a5 5 0 0 1-7-7L14 4a3.4 3.4 0 0 1 4.8 4.8l-8.6 8.6a1.7 1.7 0 0 1-2.4-2.4L16 7"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.6-3.6"/>',
    pen: '<path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z"/><path d="m13.5 6.5 4 4"/>',
    sync: '<path d="M20 11a8 8 0 0 0-14-4.6L4 8"/><path d="M4 3v5h5"/><path d="M4 13a8 8 0 0 0 14 4.6l2-1.6"/><path d="M20 21v-5h-5"/>',
    contacts: '<circle cx="9" cy="8" r="3.5"/><path d="M2.5 20a6.5 6.5 0 0 1 13 0"/><path d="M16 4.5a3.5 3.5 0 0 1 0 7"/><path d="M18 14a6.5 6.5 0 0 1 3.5 6"/>',
    reply: '<path d="M9 15 4 10l5-5"/><path d="M4 10h10a6 6 0 0 1 6 6v3"/>',
    replyAll: '<path d="M8 15 3 10l5-5"/><path d="M12 15 7 10l5-5"/><path d="M7 10h7a6 6 0 0 1 6 6v3"/>',
    forward: '<path d="m15 15 5-5-5-5"/><path d="M20 10H10a6 6 0 0 0-6 6v3"/>',
    flag: '<path d="M5 21V4"/><path d="M5 4h11l-2 4 2 4H5"/>',
    folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
    shield: '<path d="M12 3 4.5 6v6c0 4.5 3.2 7.8 7.5 9 4.3-1.2 7.5-4.5 7.5-9V6z"/><path d="m8.8 12.2 2.2 2.2 4.4-4.6"/>',
    image: '<rect x="3" y="4" width="18" height="16" rx="2.5"/><circle cx="9" cy="9.5" r="1.8"/><path d="m21 16-5-5-9 9"/>',
    bell: '<path d="M6 16V11a6 6 0 0 1 12 0v5l2 2H4z"/><path d="M10 21a2 2 0 0 0 4 0"/>',
    filter: '<path d="M3 5h18l-7 8v6l-4 2v-8z"/>',
    check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
    cloudOff: '<path d="m3 3 18 18"/><path d="M8 7.5A6 6 0 0 1 18 11a4 4 0 0 1 2.4 6.8"/><path d="M17 19H7a5 5 0 0 1-1.6-9.7"/>',
    wifi: '<path d="M2 9a15 15 0 0 1 20 0"/><path d="M5.5 12.5a10 10 0 0 1 13 0"/><path d="M9 16a5 5 0 0 1 6 0"/><circle cx="12" cy="19.5" r=".8"/>',
    mail: '<rect x="3" y="5" width="18" height="14" rx="2.5"/><path d="m4 7 8 6 8-6"/>',
    lock: '<rect x="4.5" y="10.5" width="15" height="10" rx="2.5"/><path d="M8 10.5V7.5a4 4 0 0 1 8 0v3"/>',
    download: '<path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M4 20h16"/>',
    upload: '<path d="M12 21V9"/><path d="m7 14 5-5 5 5"/><path d="M4 4h16"/>',
    file: '<path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/>',
    sparkle: '<path d="M12 3c.6 4.2 2.8 6.4 7 7-4.2.6-6.4 2.8-7 7-.6-4.2-2.8-6.4-7-7 4.2-.6 6.4-2.8 7-7z"/>',
    undo: '<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/>',
    redo: '<path d="m15 14 5-5-5-5"/><path d="M20 9H9.5a5.5 5.5 0 0 0 0 11H13"/>',
    chevron: '<path d="m9 6 6 6-6 6"/>',
    tray: '<path d="M4 15h4l1.5 2.5h5L16 15h4"/><rect x="3" y="4" width="18" height="16" rx="3"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    x: '<path d="M6 6l12 12M18 6 6 18"/>',
    eye: '<path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
    eyeOff: '<path d="m3 3 18 18"/><path d="M10.6 5.1A10 10 0 0 1 12 5c6.4 0 10 7 10 7a17 17 0 0 1-3 3.9"/><path d="M6.6 6.6A17 17 0 0 0 2 12s3.6 7 10 7a9.8 9.8 0 0 0 5.4-1.6"/>',
    terminal: '<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="m7 9 3 3-3 3"/><path d="M13 15h4"/>',
    chip: '<rect x="6" y="6" width="12" height="12" rx="2"/><path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4"/>',
    snow: '<path d="M12 2v20M4.9 4.9l14.2 14.2M2 12h20M4.9 19.1 19.1 4.9"/>',
  };
  function icon(name, cls) {
    var d = P[name];
    if (!d) throw new Error("ElyKit.icon: unknown icon " + name);
    return '<svg class="ico ' + (cls || "") + '" viewBox="0 0 24 24" aria-hidden="true">' + d + "</svg>";
  }

  /* Sample mail, the same in every scene. */
  var MAIL = [
    { from: "周雨桐", av: "周", c: ["#ff8fb8", "#d9468f"], subj: "周五的设计评审改到下午三点", prev: "大家好，评审挪到 15:00，记得带上新版原型和动效稿……", time: "10:42", unread: true, clip: true, acct: "gmail" },
    { from: "GitHub", av: "G", c: ["#9aa4b8", "#4b5468"], subj: "[ElyOffice] Release v0.1.10", prev: "ElyMail ships in this release, plus fixes across the suite.", time: "09:15", unread: true, acct: "gmail" },
    { from: "Marco Rossi", av: "M", c: ["#ffc07a", "#e07a2a"], subj: "Re: Lisbon offsite photos", prev: "Here are the best shots from Thursday, the sunset ones are…", time: "昨天", star: true, clip: true, acct: "outlook" },
    { from: "林晓", av: "林", c: ["#8fe3b0", "#2e9e5b"], subj: "周末去爬山吗？", prev: "天气预报说周六晴，八点在地铁站集合，带点水和零食～", time: "昨天", unread: true, acct: "icloud" },
    { from: "Kettle & Co.", av: "K", c: ["#b6a0ff", "#6a4fd8"], subj: "Your order #48213 has shipped", prev: "Good news! Your package is on its way. Track it here…", time: "周二", acct: "gmail" },
    { from: "陈默", av: "陈", c: ["#7cc4ff", "#1e8fe0"], subj: "Q3 季度报表终稿", prev: "附件是 Q3 报表，数字已经和财务核对过，请查收。", time: "周一", clip: true, acct: "outlook" },
    { from: "Design Weekly", av: "D", c: ["#ff9b8f", "#e0344f"], subj: "本周精选：流体动效的 12 个技巧", prev: "从粘连到拉丝，这期我们聊聊怎样让界面像液体一样……", time: "周一", acct: "icloud" },
    { from: "Aiko Tanaka", av: "A", c: ["#ffe08a", "#d9a400"], subj: "Coffee next week?", prev: "I'll be in town Tuesday through Friday, any morning works!", time: "10月18日", acct: "gmail" },
  ];
  var ACCTS = [
    { id: "gmail", name: "Gmail", addr: "lin.studio@gmail.com", c: "#ea4b5a" },
    { id: "outlook", name: "Microsoft", addr: "lin@outlook.com", c: "#2f8cf0" },
    { id: "icloud", name: "iCloud", addr: "lin@icloud.com", c: "#8fb4ff" },
  ];

  function avatar(m) {
    return '<div class="mw-av" style="background:linear-gradient(150deg,' + m.c[0] + "," + m.c[1] + ')">' + m.av + "</div>";
  }
  function row(m, i, sel) {
    var flags = "";
    if (m.clip) flags += icon("clip");
    if (m.star) flags += icon("star", "star");
    return (
      '<div class="mw-row' + (m.unread ? " unread" : "") + (i === sel ? " sel" : "") + '" data-i="' + i + '">' +
      avatar(m) +
      '<div class="mw-rt"><div class="mw-r1"><div class="mw-from">' + m.from + '</div><div class="mw-flags">' + flags + '</div><div class="mw-time">' + m.time + "</div></div>" +
      '<div class="mw-subj">' + m.subj + '</div><div class="mw-prev">' + m.prev + "</div></div></div>"
    );
  }
  function box(ic, label, n, on, extra) {
    return '<div class="mw-box' + (on ? " on" : "") + '"' + (extra || "") + ">" + icon(ic) + "<span>" + label + "</span>" + (n ? '<span class="n">' + n + "</span>" : "") + "</div>";
  }

  /* Build the canonical ElyMail window inside `el` (absolutely positioned at the
     canonical frame unless opts.style overrides). Returns element refs. */
  function mailWindow(el, opts) {
    opts = opts || {};
    var pre = opts.id || "mw";
    var sel = opts.select == null ? 0 : opts.select;
    var html =
      '<div class="glass mw" id="' + pre + '-win">' +
      '<div class="mw-bar"><div class="mw-dots"><i></i><i></i><i></i></div>' +
      '<div class="mw-brand"><img src="assets/icons/ely-mail.svg" alt=""/>ElyMail</div>' +
      '<div class="mw-search" id="' + pre + '-search">' + icon("search") + '<span class="q" id="' + pre + '-q"></span><span class="ph" id="' + pre + '-ph">搜索全部邮件</span></div>' +
      '<div class="mw-spacer"></div><div class="mw-sync" id="' + pre + '-sync">' + icon("sync") + "</div>" +
      '<div class="pill blue mw-compose" id="' + pre + '-compose">' + icon("pen") + "写邮件</div></div>" +
      '<div class="mw-side" id="' + pre + '-side">' +
      '<div class="mw-sec">统一</div>' +
      box("inbox", "全部收件箱", "12", (opts.box || "all") === "all", ' id="' + pre + '-all"') +
      box("draft", "草稿", "2", opts.box === "drafts", ' id="' + pre + '-drafts"') +
      box("outbox", "发件箱", opts.outbox == null ? "" : String(opts.outbox), opts.box === "outbox", ' id="' + pre + '-outbox"') +
      box("contacts", "联系人", "", opts.box === "contacts", ' id="' + pre + '-contacts"') +
      ACCTS.map(function (a) {
        return (
          '<div class="mw-sec" id="' + pre + "-acct-" + a.id + '"><span class="acct-dot" style="background:' + a.c + '"></span>' + a.name + "</div>" +
          (a.id === "gmail"
            ? box("inbox", "收件箱", "7") + box("send", "已发送") + box("archive", "归档")
            : box("inbox", "收件箱", a.id === "outlook" ? "3" : "2"))
        );
      }).join("") +
      "</div>" +
      '<div class="mw-list" id="' + pre + '-list"><div class="mw-filters">' +
      '<div class="mw-chip">' + icon("mail") + "未读</div>" +
      '<div class="mw-chip">' + icon("star") + "已标记</div>" +
      '<div class="mw-chip">' + icon("clip") + "带附件</div></div>" +
      '<div class="mw-rows" id="' + pre + '-rows">' + MAIL.map(function (m, i) { return row(m, i, sel); }).join("") + "</div></div>" +
      '<div class="mw-read" id="' + pre + '-read">' + reader(MAIL[sel] || MAIL[0]) + "</div>" +
      "</div>";
    el.insertAdjacentHTML("beforeend", html);
    var q = function (s) { return el.querySelector(s); };
    return {
      win: q("#" + pre + "-win"),
      side: q("#" + pre + "-side"),
      list: q("#" + pre + "-list"),
      rows: el.querySelectorAll("#" + pre + "-rows .mw-row"),
      read: q("#" + pre + "-read"),
      search: q("#" + pre + "-search"),
      query: q("#" + pre + "-q"),
      placeholder: q("#" + pre + "-ph"),
      compose: q("#" + pre + "-compose"),
      sync: q("#" + pre + "-sync"),
    };
  }
  function reader(m) {
    return (
      '<div class="mw-acts"><div class="mw-act">' + icon("reply") + "回复</div><div class=\"mw-act\">" + icon("replyAll") + "全部回复</div><div class=\"mw-act\">" + icon("forward") + "转发</div><div class=\"mw-act\">" + icon("archive") + "归档</div><div class=\"mw-act\">" + icon("trash") + "</div></div>" +
      '<div class="mw-h">' + m.subj + "</div>" +
      '<div class="mw-meta">' + avatar(m) + '<div><div class="who">' + m.from + '</div><div class="addr">发给 我、设计组</div></div><div class="when">今天 ' + (m.time.indexOf(":") > 0 ? m.time : "09:30") + "</div></div>" +
      '<div class="mw-body"><p>大家好！周五的设计评审挪到下午 15:00，地点不变，还是三楼的大会议室。</p>' +
      "<p>请把新版原型和动效稿提前发到群里，尤其是邮件列表的滑动和收件箱的过渡，我们想重点看看手感。</p>" +
      '<div class="mw-file"><div class="ext">PDF</div><div>评审议程.pdf<small>248 KB</small></div></div></div>'
    );
  }

  window.ElyKit = { icon: icon, MAIL: MAIL, ACCTS: ACCTS, mailWindow: mailWindow, avatar: avatar, row: row, reader: reader };
})();
