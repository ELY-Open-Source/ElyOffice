//! Korean mail drafts messages.

pub(super) const KO: &[(&str, &str)] = &[
    ("mail.boxes.drafts", "임시 보관함"),
    ("mail.drafts.pick", "볼 임시 보관 메시지를 고르십시오"),
    ("mail.drafts.to", "받는 사람"),
    ("mail.drafts.cc", "참조"),
    ("mail.drafts.bcc", "숨은 참조"),
    ("mail.drafts.saved", "저장됨"),
    ("mail.drafts.continue", "이어서 쓰기"),
    ("mail.drafts.delete", "임시 보관 메시지 삭제…"),
    ("mail.drafts.kept", "메시지를 임시 보관함에 보관했습니다"),
    ("mail.drafts.unkept", "임시 보관 메시지를 저장하지 못했습니다: {error}"),
    ("mail.drafts.undropped", "임시 보관 메시지를 삭제하지 못했습니다: {error}"),
    ("mail.drafts.unread", "임시 보관함을 읽을 수 없습니다: {error}"),
    ("mail.drafts.up", "서버의 임시 보관함 폴더에도 보관"),
    ("mail.drafts.unsaved_choice", "임시 보관 설정을 저장하지 못했습니다: {error}"),
    ("mail.drafts.no_folder", "이 컴퓨터에 보관했습니다. 서버에 아직 임시 보관함 폴더가 없습니다."),
    ("mail.drafts.server_failed", "이 컴퓨터에 보관했습니다. 서버 사본은 저장하지 못했습니다: {error}"),
    ("mail.drafts.server_stale", "서버의 임시 보관함에 이전 사본이 남아 있을 수 있습니다: {error}"),
];
