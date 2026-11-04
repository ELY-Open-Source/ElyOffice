//! Korean mail notify messages.

pub(super) const KO: &[(&str, &str)] = &[
    ("mail.notify.title", "새 메일"),
    ("mail.notify.body.one", "보낸 사람: {senders}"),
    ("mail.notify.body", "{senders}의 새 메시지 {count}개"),
    ("mail.notify.more", "{senders} 외"),
    ("mail.file.downloading", "메시지와 파일을 다운로드하는 중…"),
    ("mail.file.download_failed", "파일을 다운로드하지 못했습니다: {error}"),
    ("mail.file.unread", "파일을 읽을 수 없습니다: {error}"),
    ("mail.file.unsaved", "파일을 저장하지 못했습니다: {error}"),
    ("mail.file.saved", "{name}을(를) 저장했습니다"),
    ("mail.file.unopened", "파일을 열지 못했습니다: {error}"),
    ("mail.file.unsafe", "이 종류의 파일은 프로그램을 실행할 수 있어 여기서 열지 않습니다. 저장한 뒤, 보낸 사람을 믿는다면 직접 여십시오."),
];
