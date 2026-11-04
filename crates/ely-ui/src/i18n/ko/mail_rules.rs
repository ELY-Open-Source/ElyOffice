//! Korean mail rules messages.

pub(super) const KO: &[(&str, &str)] = &[
    ("mail.rules.field", "확인할 곳"),
    ("mail.rules.from", "보낸 사람"),
    ("mail.rules.to", "받는 사람 또는 참조"),
    ("mail.rules.subject", "제목"),
    ("mail.rules.words", "찾을 단어"),
    ("mail.rules.action", "동작"),
    ("mail.rules.flag", "플래그 지정"),
    ("mail.rules.archive", "보관"),
    ("mail.rules.move", "{name}(으)로 이동"),
    ("mail.rules.add", "규칙 추가"),
    ("mail.rules.remove", "규칙 제거"),
    ("mail.rules.said", "{field}에 “{words}”이(가) 있는 새 메일: {action}"),
    ("mail.rules.unsaved", "규칙을 저장하지 못했습니다: {error}"),
    ("mail.rules.unread", "규칙을 읽을 수 없습니다: {error}"),
    ("mail.rules.refused", "규칙을 실행하지 못했습니다: {error}"),
];
