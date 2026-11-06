//! ElyMail's message body menu.

pub(super) const EN: &[(&str, &str)] = &[
    ("mail.body.copy", "Copy"),
    ("mail.body.copy_link", "Copy Link Address"),
    ("mail.body.copy_picture", "Copy Picture"),
    ("mail.body.save_picture", "Save Picture As…"),
    ("mail.body.blocked", "Load the pictures above to copy or save this one."),
    ("mail.body.uncopied", "The picture was not copied: {error}"),
    ("mail.body.unsaved", "The picture was not saved: {error}"),
    ("mail.body.picture_name", "picture"),
];

pub(super) const ZH_CN: &[(&str, &str)] = &[
    ("mail.body.copy", "复制"),
    ("mail.body.copy_link", "复制链接地址"),
    ("mail.body.copy_picture", "复制图片"),
    ("mail.body.save_picture", "图片另存为…"),
    ("mail.body.blocked", "先载入上方的图片，才能复制或保存这张。"),
    ("mail.body.uncopied", "图片没能复制：{error}"),
    ("mail.body.unsaved", "图片没能保存：{error}"),
    ("mail.body.picture_name", "图片"),
];
