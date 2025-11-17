# Status
- ✅ Forms (`src/03-basic/05-form.md`) and Lists (`src/03-basic/06-list.md`) chapters now pass `PATH="$PWD/scripts:$PATH" mdbook test`. Examples import `waterui_form::FormBuilder` + `waterui_derive::Project`, annotate `Binding` types, and rely on `waterui::prelude::*`. The custom `scripts/rustdoc` adds `--edition=2021` when missing and feeds rustdoc every local WaterUI crate (including `waterui-derive`).
- ❌ Remaining doctest failures live in: `03-basic/07-media.md`, `03-basic/08-navigation.md`, `03-basic/09-gesture-and-haptic.md`, `03-basic/10-web-request.md`, and the entire `04-advanced` section (animation, suspense, error handling, plugin, accessibility, i18n, shader). Latest log: `/tmp/mdbook_latest.log` (generated via `PATH="$PWD/scripts:$PATH" mdbook test`).

# Next Actions
1. **Media chapter (`src/03-basic/07-media.md`)**
   - Replace `waterui::components::media` imports with `waterui::media::*`.
   - Ensure `Binding<bool>`/`Binding<Option<...>>` types are explicit for `action_with`.
   - Photo placeholders must wrap in `AnyView::new`.
   - Live Photo & Picker sections need proper async handling via `task(view, &source, handler)` or simplified sync stubs.

2. **Navigation chapter (`src/03-basic/08-navigation.md`)**
   - Add `use waterui::prelude::*; use waterui::navigation::*;`.
   - Provide concrete helper functions (`settings_list`, `profile_detail`, etc.) so samples compile.
   - `NavigationLink`, `navigation`, `tabs` examples should capture bindings when calling `action_with`.

3. **Gesture & Haptics (`src/03-basic/09-gesture-and-haptic.md`)**
   - Import `waterui::gesture::{GestureObserver, TapGesture, DragGesture, HapticFeedback, ...}` and `waterui::core::extract::Use`.
   - Add `Environment` + plugin stubs for haptics.

4. **Fetching Data (`src/03-basic/10-web-request.md`)**
   - Provide a minimal `FetchState` enum and fake fetcher (avoid real `reqwest` if network disabled—stub response).
   - Use `task(view, &state, move |env, ctx| ...)` signature correctly.
   - Fix `Suspense::new` usage (`Suspense::new(content).loading(...)`).

5. **Advanced chapters**
   - **Animation (`04-advanced/01-animation.md`)**: import `binding`, `text`, `waterui::animation::Animation`; wrap `.with_animation` outputs in `Computed`/`Binding`.
   - **Suspense (`04-advanced/02-suspense.md`)**: import `Environment`, `task`, `Suspense`, `binding`; supply simple loaders/error handlers.
   - **Error handling (`04-advanced/03-error-handling.md`)**: bring in `AnyView`, `ErrorView`, `ErrorViewBuilder`, `button`, `task`; stub `Telemetry`, `retry_last_request`, `app_root`.
   - **Plugin (`04-advanced/04-plugin.md`)**: import `waterui_core::plugin::Plugin`, `Environment`; switch i18n example to `waterui_text` APIs.
   - **Accessibility (`04-advanced/05-accessibility.md`)**: import `View`, `vstack`, `AccessibilityState`; implement helper functions for state updates.
   - **i18n (`04-advanced/06-i18n.md`)**: replace `waterui_i18n` with `waterui_text::locale::*`, fix `text!` capture syntax and `action_with` types.
   - **Shader (`04-advanced/08-shader.md`)**: use `waterui::graphics::Shader` (or actual module), ensure `.animated()` result fits expected type (store as `Computed<f32>`).

6. **Validation loop**
   - After each chapter fix, run `PATH="$PWD/scripts:$PATH" mdbook test`.
   - Keep `/tmp/mdbook_latest.log` updated for quick failure reference.
   - Once doctests pass, consider `cargo test` if requested.
