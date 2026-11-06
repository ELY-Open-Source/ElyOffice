//! Korean mail body menu messages.

pub(super) const KO: &[(&str, &str)] = &[
    ("mail.body.copy", "복사"),
    ("mail.body.copy_link", "링크 주소 복사"),
    ("mail.body.copy_picture", "이미지 복사"),
    ("mail.body.save_picture", "다른 이름으로 이미지 저장…"),
    ("mail.body.blocked", "이 이미지를 복사하거나 저장하려면 먼저 위의 이미지를 불러오세요."),
    ("mail.body.uncopied", "이미지를 복사하지 못했습니다: {error}"),
    ("mail.body.unsaved", "이미지를 저장하지 못했습니다: {error}"),
    ("mail.body.picture_name", "이미지"),
];
