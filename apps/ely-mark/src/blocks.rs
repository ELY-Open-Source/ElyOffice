//! Typora's Paragraph and Format menus at the caret (MARK-15).

use ely_gpui_component::secondary;
use ely_gpui_component::{
    documents::{BlockCommand, EditRefusal, InlineFormat, MarkdownDocument, TableRefusal},
    primitives::Severity,
};
use ely_office::prelude::*;
use ely_ui::{commands, i18n::text};
use gpui::{
    Action, App, Context, InteractiveElement as _, KeyBinding, Menu, MenuItem, SharedString,
    actions,
};

use crate::{desk::Desk, keys::item};

actions!(
    mark,
    [
        Heading1,
        Heading2,
        Heading3,
        Heading4,
        Heading5,
        Heading6,
        Paragraph,
        Promote,
        Demote,
        MathBlock,
        CodeFences,
        Quote,
        OrderedList,
        BulletList,
        TaskList,
        ParagraphBefore,
        ParagraphAfter,
        LinkReference,
        Footnote,
        Rule,
        Toc,
        FrontMatter,
        Strong,
        Emphasis,
        Underline,
        InlineCode,
        Strike,
        Hyperlink,
        InsertImage,
        ClearFormat,
        DeleteBlock
    ]
);

pub fn bind(cx: &mut App) {
    let at = Some(commands::CONTEXT);
    cx.bind_keys([
        KeyBinding::new(secondary!("1"), Heading1, at),
        KeyBinding::new(secondary!("2"), Heading2, at),
        KeyBinding::new(secondary!("3"), Heading3, at),
        KeyBinding::new(secondary!("4"), Heading4, at),
        KeyBinding::new(secondary!("5"), Heading5, at),
        KeyBinding::new(secondary!("6"), Heading6, at),
        KeyBinding::new(secondary!("0"), Paragraph, at),
        KeyBinding::new(secondary!("="), Promote, at),
        KeyBinding::new(secondary!("-"), Demote, at),
        KeyBinding::new(secondary!("shift-m"), MathBlock, at),
        KeyBinding::new(secondary!("shift-k"), CodeFences, at),
        KeyBinding::new(secondary!("shift-q"), Quote, at),
        KeyBinding::new(secondary!("{"), OrderedList, at),
        KeyBinding::new(secondary!("}"), BulletList, at),
        KeyBinding::new(secondary!("shift-x"), TaskList, at),
        KeyBinding::new(secondary!("b"), Strong, at),
        KeyBinding::new(secondary!("i"), Emphasis, at),
        KeyBinding::new(secondary!("u"), Underline, at),
        KeyBinding::new(secondary!("~"), InlineCode, at),
        KeyBinding::new("alt-%", Strike, at),
        KeyBinding::new(secondary!("k"), Hyperlink, at),
        KeyBinding::new(secondary!("shift-i"), InsertImage, at),
        KeyBinding::new(secondary!("\\"), ClearFormat, at),
    ]);
}

pub fn paragraph_menu(cx: &App) -> Menu {
    let item = |key: &str, action: Box<dyn Action>| item(key, action, cx);
    Menu::new(text(cx, "mark.menu.paragraph", &[])).items([
        item("mark.block.h1", Box::new(Heading1)),
        item("mark.block.h2", Box::new(Heading2)),
        item("mark.block.h3", Box::new(Heading3)),
        item("mark.block.h4", Box::new(Heading4)),
        item("mark.block.h5", Box::new(Heading5)),
        item("mark.block.h6", Box::new(Heading6)),
        MenuItem::separator(),
        item("mark.block.paragraph", Box::new(Paragraph)),
        MenuItem::separator(),
        item("mark.block.promote", Box::new(Promote)),
        item("mark.block.demote", Box::new(Demote)),
        MenuItem::separator(),
        MenuItem::submenu(crate::tools::table_menu(cx)),
        item("mark.block.math", Box::new(MathBlock)),
        item("mark.block.code", Box::new(CodeFences)),
        MenuItem::separator(),
        item("mark.block.quote", Box::new(Quote)),
        MenuItem::separator(),
        item("mark.block.ordered", Box::new(OrderedList)),
        item("mark.block.bullet", Box::new(BulletList)),
        item("mark.block.task", Box::new(TaskList)),
        MenuItem::separator(),
        item("mark.block.before", Box::new(ParagraphBefore)),
        item("mark.block.after", Box::new(ParagraphAfter)),
        MenuItem::separator(),
        item("mark.block.link_reference", Box::new(LinkReference)),
        item("mark.block.footnote", Box::new(Footnote)),
        MenuItem::separator(),
        item("mark.block.rule", Box::new(Rule)),
        item("mark.block.toc", Box::new(Toc)),
        item("mark.block.front_matter", Box::new(FrontMatter)),
    ])
}

pub fn format_menu(cx: &App) -> Menu {
    let item = |key: &str, action: Box<dyn Action>| item(key, action, cx);
    Menu::new(text(cx, "mark.menu.format", &[])).items([
        item("mark.format.strong", Box::new(Strong)),
        item("mark.format.emphasis", Box::new(Emphasis)),
        item("mark.format.underline", Box::new(Underline)),
        item("mark.format.code", Box::new(InlineCode)),
        MenuItem::separator(),
        item("mark.format.strike", Box::new(Strike)),
        MenuItem::separator(),
        item("mark.format.link", Box::new(Hyperlink)),
        item("mark.format.image", Box::new(InsertImage)),
        MenuItem::separator(),
        item("mark.format.clear", Box::new(ClearFormat)),
    ])
}

/// What to tell the user for `why`.
pub(crate) fn refusal(why: EditRefusal, cx: &App) -> SharedString {
    let key = match why {
        EditRefusal::Preview => "mark.preview.read_only",
        EditRefusal::Code => "mark.refused.code",
        EditRefusal::Math => "mark.refused.math",
        EditRefusal::Html => "mark.refused.html",
        EditRefusal::FrontMatter => "mark.refused.front_matter",
        EditRefusal::Outside => "mark.refused.outside",
        EditRefusal::Table => "mark.refused.table",
        EditRefusal::NoImage => "mark.refused.no_image",
        EditRefusal::Present => "mark.refused.present",
        EditRefusal::Across => "mark.refused.across",
        EditRefusal::TooLong => "mark.refused.too_long",
        EditRefusal::Busy => "mark.refused.busy",
        EditRefusal::Changed => "mark.refused.changed",
        EditRefusal::Misread => "mark.refused.misread",
        EditRefusal::Cells(TableRefusal::Outside) => "mark.table.none",
        EditRefusal::Cells(TableRefusal::Kept) => "mark.table.kept",
        EditRefusal::Cells(TableRefusal::Beyond) => "mark.table.beyond",
        EditRefusal::Cells(TableRefusal::Mixed) => "mark.table.mixed",
        EditRefusal::Cells(TableRefusal::Edge) => "mark.table.edge",
    };
    text(cx, key, &[])
}

/// Each action to its document edit.
macro_rules! edits {
    ($root:expr, $cx:expr, $($action:ident => $edit:expr),* $(,)?) => {
        $root$(.on_action($cx.listener(|desk, _: &$action, _, cx| desk.edit($edit, cx))))*
    };
}
pub(crate) use edits;

impl Desk {
    /// `edit` on the shown document; a refusal is said.
    pub(crate) fn edit(
        &mut self,
        edit: impl FnOnce(
            &mut MarkdownDocument,
            &mut Context<MarkdownDocument>,
        ) -> Result<(), EditRefusal>,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        let document = view.read(cx).document().clone();
        if let Err(why) = document.update(cx, edit) {
            self.toast(refusal(why, cx), Severity::Info, cx);
        }
    }

    pub fn block_actions(root: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        use BlockCommand as B;
        use InlineFormat as F;
        edits!(root, cx,
            Heading1 => |d, cx| d.block(B::Heading(1), cx),
            Heading2 => |d, cx| d.block(B::Heading(2), cx),
            Heading3 => |d, cx| d.block(B::Heading(3), cx),
            Heading4 => |d, cx| d.block(B::Heading(4), cx),
            Heading5 => |d, cx| d.block(B::Heading(5), cx),
            Heading6 => |d, cx| d.block(B::Heading(6), cx),
            Paragraph => |d, cx| d.block(B::Paragraph, cx),
            Promote => |d, cx| d.block(B::Promote, cx),
            Demote => |d, cx| d.block(B::Demote, cx),
            MathBlock => |d, cx| d.block(B::MathBlock, cx),
            CodeFences => |d, cx| d.block(B::CodeBlock, cx),
            Quote => |d, cx| d.block(B::Quote, cx),
            OrderedList => |d, cx| d.block(B::OrderedList, cx),
            BulletList => |d, cx| d.block(B::BulletList, cx),
            TaskList => |d, cx| d.block(B::TaskList, cx),
            ParagraphBefore => |d, cx| d.block(B::ParagraphBefore, cx),
            ParagraphAfter => |d, cx| d.block(B::ParagraphAfter, cx),
            LinkReference => |d, cx| d.block(B::LinkDefinition, cx),
            Footnote => |d, cx| d.block(B::Footnote, cx),
            Rule => |d, cx| d.block(B::Rule, cx),
            Toc => |d, cx| d.block(B::Toc, cx),
            FrontMatter => |d, cx| d.block(B::FrontMatter, cx),
            Strong => |d, cx| d.format(F::Strong, cx),
            Emphasis => |d, cx| d.format(F::Emphasis, cx),
            Underline => |d, cx| d.format(F::Underline, cx),
            InlineCode => |d, cx| d.format(F::Code, cx),
            Strike => |d, cx| d.format(F::Strike, cx),
            Hyperlink => |d, cx| d.format(F::Link, cx),
            InsertImage => |d, cx| d.format(F::Image, cx),
            ClearFormat => |d, cx| d.format(F::Clear, cx),
            DeleteBlock => |d, cx| d.block(B::Delete, cx),
        )
    }
}

// macOS menus are the system's, not drawn.
#[cfg(test)]
mod context;
#[cfg(all(test, not(target_os = "macos")))]
mod menus;
#[cfg(test)]
mod tests;
