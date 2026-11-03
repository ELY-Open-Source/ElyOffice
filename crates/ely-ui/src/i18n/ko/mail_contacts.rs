//! Korean mail contacts messages.

pub(super) const KO: &[(&str, &str)] = &[
    ("mail.boxes.contacts", "연락처"),
    ("mail.contacts.name", "이름"),
    ("mail.contacts.address", "이메일 주소"),
    ("mail.contacts.add", "연락처 추가"),
    ("mail.contacts.remove", "연락처 제거"),
    ("mail.contacts.import", "vCard 가져오기…"),
    ("mail.contacts.export", "vCard 내보내기…"),
    ("mail.contacts.empty", "아직 연락처가 없습니다. 메일을 보내는 상대가 여기에 추가됩니다."),
    ("mail.contacts.unread", "연락처를 읽을 수 없습니다: {error}"),
    ("mail.contacts.unsaved", "연락처를 저장하지 못했습니다: {error}"),
    ("mail.contacts.imported", "가져옴: {added}개 추가, {renamed}개 이름 바뀜, {unchanged}개 이미 있음, 카드 {skipped}개는 주소가 없었습니다"),
    ("mail.contacts.unimported", "vCard 파일을 가져오지 못했습니다: {error}"),
    ("mail.contacts.unexported", "연락처를 내보내지 못했습니다: {error}"),
];
