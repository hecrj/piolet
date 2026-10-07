use crate::font;
use crate::locale;

use iced::border;
use iced::widget::operation;
use iced::widget::{self, button, column, container, scrollable, text};
use iced::{Theme, Widget};
use iced_palace::widget::capsule;

pub fn snap<Message>(
    animation: operation::Animation,
    scroll: scrollable::Scroll,
) -> scrollable::Action<Message> {
    const MAX_DISTANCE: f32 = 60.0;

    let Some(origin) = scroll.origin else {
        return scrollable::Action::ScrollTo(
            scroll.viewport.end().into(),
            operation::Animation::Instant,
        );
    };

    if matches!(
        scroll.source,
        scrollable::Source::Scrollbar
            | scrollable::Source::AutoScroll
            | scrollable::Source::Operation
            | scrollable::Source::Wheel
    ) {
        return scrollable::Action::None;
    }

    let destination = scroll.destination();

    if origin.slide(destination).distance_to_end().y > MAX_DISTANCE
        || destination.absolute_offset().y + 1.0 < scroll.viewport.absolute_offset().y
    {
        return scrollable::Action::None;
    }

    scrollable::Action::ScrollTo(
        scroll.viewport.end().into(),
        if destination.distance_to_end().y > 2000.0 {
            operation::Animation::Instant
        } else {
            animation
        },
    )
}

pub fn context_led<Message: 'static>(
    context_size: Option<u64>,
    timings: Option<reason::Timings>,
) -> impl Widget<Message> {
    use iced::mouse;
    use iced::widget::{canvas, tooltip};
    use iced::{Radians, Rectangle, Renderer};

    use std::cell::RefCell;
    use std::f32::consts::{FRAC_PI_2, PI};

    const SIZE: f32 = 14.0;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    struct Led {
        context_size: Option<u64>,
        timings: Option<reason::Timings>,
    }

    #[derive(Default)]
    struct State {
        last: RefCell<Led>,
        cache: canvas::Cache,
    }

    impl<Message> canvas::Program<Message> for Led {
        type State = State;

        fn draw(
            &self,
            state: &Self::State,
            renderer: &Renderer,
            theme: &Theme,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            const STROKE_WIDTH: f32 = 2.0;

            if *state.last.borrow() != *self {
                *state.last.borrow_mut() = *self;
                state.cache.clear();
            }

            let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
                let palette = theme.palette();
                let radius = (frame.width() - STROKE_WIDTH) / 2.0;
                let circle = canvas::Path::circle(frame.center(), radius);

                frame.stroke(
                    &circle,
                    canvas::Stroke {
                        style: canvas::Style::Solid(palette.background.strong.color),
                        width: STROKE_WIDTH,
                        ..canvas::Stroke::default()
                    },
                );

                if let Some(timings) = self.timings
                    && let Some(context_size) = self.context_size
                {
                    let usage = timings.total_tokens() as f32 / context_size as f32;

                    let arc = {
                        let mut builder = canvas::path::Builder::new();

                        let start = -FRAC_PI_2;

                        builder.arc(canvas::path::Arc {
                            center: frame.center(),
                            radius,
                            start_angle: Radians(start),
                            end_angle: Radians(start + 2.0 * PI * usage),
                        });

                        builder.build()
                    };

                    frame.stroke(
                        &arc,
                        canvas::Stroke {
                            style: canvas::Style::Solid(match usage {
                                0.0..0.8 => palette.primary.base.color,
                                0.8..0.9 => palette.warning.base.color,
                                _ => palette.danger.base.color,
                            }),
                            width: STROKE_WIDTH,
                            line_cap: canvas::LineCap::Square,
                            ..canvas::Stroke::default()
                        },
                    );
                }
            });

            vec![geometry]
        }
    }

    let led = canvas(Led {
        timings,
        context_size,
    })
    .width(SIZE)
    .height(SIZE);

    match (context_size, timings) {
        (Some(context_size), Some(timings)) => {
            let tokens = timings.total_tokens();
            let percent = tokens as f32 / context_size as f32 * 100.0;

            tooltip(
                led,
                container(
                    text!(
                        "{} / {} ({percent:.1}%)",
                        locale::thousands(tokens),
                        locale::thousands(context_size)
                    )
                    .size(font::TINY),
                )
                .padding(5)
                .style(container::rounded_box),
            )
            .position(tooltip::Position::Top)
            .boxed()
        }
        _ => led.boxed(),
    }
}

pub fn collapsible<'a, Message, A, B>(
    force_open: bool,
    base: impl Fn(bool) -> A + 'a,
    content: impl Fn() -> B + 'a,
) -> impl Widget<Message> + 'a
where
    Message: Clone + 'static,
    A: Widget<Message> + 'a,
    B: Widget<Message> + 'a,
{
    capsule(move |open| {
        let open = force_open || open;

        column![
            button(base(open).map(capsule::Event::Message))
                .on_press_maybe((!force_open).then_some(capsule::Event::Set(!open)))
                .padding(0)
                .style(move |theme: &Theme, status| button::Style {
                    text_color: if (open && status != button::Status::Pressed)
                        || status == button::Status::Hovered
                    {
                        theme.seed().text
                    } else {
                        theme.palette().secondary.strong.color
                    },
                    ..button::Style::default()
                }),
            open.then(&content)
                .map(|content| content.map(capsule::Event::Message))
        ]
    })
}

pub fn popover<'a, Message, A, B>(
    base: impl Fn() -> A + 'a,
    popup: impl Fn() -> B + 'a,
) -> impl Widget<Message> + 'a
where
    Message: Clone + 'static,
    A: Widget<Message> + 'a,
    B: Widget<Message> + 'a,
{
    capsule(move |open: bool| {
        widget::popover(
            button(base().map(capsule::Event::Message))
                .on_press(capsule::Event::Set(!open))
                .padding([4, 2])
                .style(move |theme: &Theme, status| {
                    if open || matches!(status, button::Status::Hovered | button::Status::Pressed) {
                        button::Style {
                            background: Some(
                                if open {
                                    theme.palette().background.weaker
                                } else {
                                    theme.palette().background.weakest
                                }
                                .color
                                .into(),
                            ),
                            border: border::rounded(5),
                            ..button::text(theme, status)
                        }
                    } else {
                        button::text(theme, status)
                    }
                }),
            open.then(|| {
                container(popup().map(capsule::Event::Message))
                    .padding(10)
                    .style(container::bordered_box)
            }),
        )
        .position(widget::popover::Position::Top)
        .gap(10)
        .on_close(capsule::Event::Set(false))
    })
}

pub fn arrow(open: bool) -> &'static str {
    if open { "⏷" } else { "⏵" }
}
