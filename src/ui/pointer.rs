// ------------------------------------------------------------
// Module: pointer
// Purpose: Records the pointer at the window root. The global
//          event listener does not receive mouse events, so the
//          menu and the drag label cannot use it.
// ------------------------------------------------------------

use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::app::{Message, PodcastMessage};

// ------------------------------------------------------------
// Struct: PointerTracker
// Purpose: Publishes the pointer position on a press, follows it
//          while a favorite is held, and ends the drag on release.
//          It does not capture the event, so the page still clicks.
// ------------------------------------------------------------
pub struct PointerTracker<'a> {
    content: Element<'a, Message>,
    held: bool,
}

pub fn tracker<'a>(content: impl Into<Element<'a, Message>>, held: bool) -> PointerTracker<'a> {
    PointerTracker {
        content: content.into(),
        held,
    }
}

impl<'a> Widget<Message, iced::Theme, iced::Renderer> for PointerTracker<'a> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // The press position is queued before the page handles the
        // click, so a right-click menu opens where the pointer is.
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = event
            && let Some(position) = cursor.position()
        {
            shell.publish(Message::CursorMoved(position));
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) if self.held => {
                if let Some(position) = cursor.position() {
                    shell.publish(Message::CursorMoved(position));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                shell.publish(Message::Podcast(PodcastMessage::DragReleased));
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a> From<PointerTracker<'a>> for Element<'a, Message> {
    fn from(tracker: PointerTracker<'a>) -> Self {
        Element::new(tracker)
    }
}
