//! Device safe-area insets, in CSS pixels: `x` is the top inset (camera
//! notch), `y` the bottom one (home indicator).

pub use backend::insets;

#[cfg(target_arch = "wasm32")]
mod backend {
    use bevy::math::Vec2;

    const PROBE: &str = "safe-area";

    /// `env(safe-area-inset-*)` is only reachable from CSS, so `index.html`
    /// parks the four values on the padding of a hidden probe element and this
    /// reads them back.
    pub fn insets() -> Vec2 {
        read().unwrap_or(Vec2::ZERO)
    }

    fn read() -> Option<Vec2> {
        let window = web_sys::window()?;
        let probe = window.document()?.get_element_by_id(PROBE)?;
        let style = window.get_computed_style(&probe).ok()??;
        Some(Vec2::new(
            px(&style, "padding-top"),
            px(&style, "padding-bottom"),
        ))
    }

    fn px(style: &web_sys::CssStyleDeclaration, property: &str) -> f32 {
        style
            .get_property_value(property)
            .ok()
            .and_then(|value| value.trim().trim_end_matches("px").parse().ok())
            .unwrap_or(0.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::env;
    use std::sync::OnceLock;

    use bevy::math::Vec2;

    const VAR: &str = "WATER_SORT_SAFE_AREA";

    /// A desktop window has no insets; the variable is how the dev channel
    /// fakes a notched phone.
    pub fn insets() -> Vec2 {
        static INSETS: OnceLock<Vec2> = OnceLock::new();
        *INSETS.get_or_init(|| {
            env::var(VAR)
                .ok()
                .and_then(|raw| parse(&raw))
                .unwrap_or(Vec2::ZERO)
        })
    }

    fn parse(raw: &str) -> Option<Vec2> {
        let (top, bottom) = raw.split_once(',')?;
        Some(Vec2::new(
            top.trim().parse().ok()?,
            bottom.trim().parse().ok()?,
        ))
    }
}
