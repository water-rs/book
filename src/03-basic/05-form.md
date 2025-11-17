# Form Controls

WaterUI provides a comprehensive form system that makes creating interactive forms both simple and powerful. The centerpiece of this system is the `FormBuilder` derive macro, which automatically generates form UIs from your data structures.

## Two-Way Data Binding

WaterUI's forms are built on a powerful concept called **two-way data binding**. This means that the state of your data model and the state of your UI controls are always kept in sync automatically.

Here's how it works:
1.  You provide a `Binding` of your data structure (e.g., `Binding<LoginForm>`) to a form control.
2.  The form control (e.g., a `TextField`) reads the initial value from the binding to display it.
3.  When the user interacts with the control (e.g., types into the text field), the control **automatically updates the value inside your original `Binding`**.

This creates a seamless, reactive loop:
-   **Model → View:** If you programmatically change the data in your `Binding`, the UI control will instantly reflect that change.
-   **View → Model:** If the user changes the value in the UI control, your underlying data `Binding` is immediately updated.

This eliminates a huge amount of boilerplate code. You don't need to write manual event handlers to update your state for every single input field. The binding handles it for you. All form components in WaterUI, whether used individually or through the `FormBuilder`, use this two-way binding mechanism.

## Quick Start with FormBuilder

The easiest way to create forms in WaterUI is combining the `Project` and `FormBuilder` derives: `#[derive(waterui_derive::Project, waterui_derive::FormBuilder)]`. `Project` gives you field-level bindings, while `FormBuilder` renders the UI automatically:

```rust
# use waterui::prelude::*;
# use waterui_form::{form, FormBuilder};
#[derive(Default, Clone, Debug, waterui_derive::Project, waterui_derive::FormBuilder)]
pub struct LoginForm {
    /// The user's username
    pub username: String,
    /// The user's password  
    pub password: String,
    /// Whether to remember the user
    pub remember_me: bool,
    /// The user's age
    pub age: i32,
}

pub fn login_view() -> impl View {
    let login_form = LoginForm::binding();
    form(&login_form)
}
```

That's it! WaterUI automatically creates appropriate form controls for each field type:

- `String` → Text field
- `bool` → Toggle switch
- `i32` → Number stepper
- `f64` → Slider
- And many more...

## Type-to-Component Mapping

The `FormBuilder` macro automatically maps Rust types to appropriate form components:

| Rust Type | Form Component | Description |
|-----------|----------------|-------------|
| `String`, `&str` | `TextField` | Single-line text input |
| `bool` | `Toggle` | On/off switch |
| `i32`, `i64`, etc. | `Stepper` | Numeric input with +/- buttons |
| `f64` | `Slider` | Slider with 0.0-1.0 range |
| `Color` | `ColorPicker` | Color selection widget |

## Complete Example: User Registration Form

Let's build a more comprehensive form:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::SignalExt;
# use waterui_form::{form, FormBuilder};
use waterui::Color;

#[derive(Default, Clone, Debug, waterui_derive::Project, waterui_derive::FormBuilder)]
struct RegistrationForm {
    /// Full name (2-50 characters)
    full_name: String,
    /// Email address
    email: String,
    /// Age (must be 18+)
    age: i32,
    /// Subscribe to newsletter
    newsletter: bool,
    /// Account type
    is_premium: bool,
    /// Profile completion (0.0 to 1.0)
    profile_completion: f64,
    /// Theme color preference
    theme_color: Color,
}

pub fn registration_view() -> impl View {
    let form_binding = RegistrationForm::binding();

    let validation_message = form_binding.clone().map(|data| -> String {
        if data.full_name.len() < 2 {
            "Name too short".into()
        } else if data.age < 18 {
            "Must be 18 or older".into()
        } else if !data.email.contains('@') {
            "Invalid email".into()
        } else {
            "Form is valid ✓".into()
        }
    });

    vstack((
        "User Registration",
        form(&form_binding),
        text!("{validation_message}"),
    ))
}
```

## Individual Form Controls

You can also use form controls individually:

### Text Fields

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
# use waterui::Str;
pub fn text_field_example() -> impl View {
    let name: Binding<String> = binding(String::new());
    let name_str = Binding::mapping(&name, |value| Str::from(value), |binding, value: Str| {
        binding.set(value.to_string());
    });
    vstack((
        text("Name:"),
        TextField::new(&name_str).prompt(text("you@example.com")),
    ))
}
```

### Toggle Switches

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
pub fn toggle_example() -> impl View {
    let enabled: Binding<bool> = binding(false);
    toggle("Enable notifications", &enabled)
}
```

### Number Steppers

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
pub fn stepper_example() -> impl View {
    let count: Binding<i32> = binding(0);
    stepper(&count)
}
```

### Sliders

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
fn slider_example() -> impl View {
    let volume: Binding<f64> = binding(0.5_f64);
    Slider::new(0.0..=1.0, &volume).label(text("Volume"))
}
```

### Color Picker

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
# use waterui::Color;
# use waterui::form::picker::ColorPicker;

fn theme_selector() -> impl View {
    let color: Binding<Color> = binding(Color::srgb_f32(0.25, 0.6, 0.95));

    ColorPicker::new(&color)
        .label(text("Theme color"))
}
```

## Advanced Form Patterns

### Multi-Step Forms

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::SignalExt;
# use waterui::widget::condition::when;
# use waterui::layout::stack::hstack;
# use waterui::layout::spacer;
# use waterui::Binding;
# use waterui_form::{form, FormBuilder};
#[derive(Default, Clone, waterui_derive::Project, waterui_derive::FormBuilder)]
struct PersonalInfo {
    first_name: String,
    last_name: String,
    birth_year: i32,
}

#[derive(Default, Clone, waterui_derive::Project, waterui_derive::FormBuilder)]
struct ContactInfo {
    email: String,
    phone: String,
    preferred_contact: bool, // true = email, false = phone
}

pub fn registration_wizard() -> impl View {
    let personal = PersonalInfo::binding();
    let contact = ContactInfo::binding();
    let current_step = binding(0_usize);
    let step_display = waterui::SignalExt::map(current_step.clone(), |value| value + 1);
    let show_personal = waterui::SignalExt::map(current_step.clone(), |step| step == 0);

    vstack((
        text!("Step {} of 2", step_display),
        when(show_personal, move || form(&personal))
        .or(move || form(&contact)),
        hstack((
            button("Back").action_with(&current_step, |state: Binding<usize>| {
                state.set(state.get().saturating_sub(1));
            }),
            spacer(),
            button("Next").action_with(&current_step, |state: Binding<usize>| {
                state.set((state.get() + 1).min(1));
            }),
        )),
    ))
}
```

### Custom Form Layouts

For complete control over form layout, implement `FormBuilder` manually:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::{AnyView, Binding, Str};

#[derive(Default, Clone)]
struct CustomForm {
    title: String,
    active: bool,
}

impl FormBuilder for CustomForm {
    type View = AnyView;

    fn view(binding: &Binding<Self>, _label: AnyView, _placeholder: Str) -> Self::View {
        let title_binding = Binding::mapping(binding, |data| Str::from(data.title.clone()), |form, value: Str| {
            form.with_mut(|state| state.title = value.to_string());
        });
        let active_binding = Binding::mapping(binding, |data| data.active, |form, value| {
            form.with_mut(|state| state.active = value);
        });

        AnyView::new(vstack((
            TextField::new(&title_binding).label(text("Title")),
            Toggle::new(&active_binding).label(text("Active")),
        )))
    }
}
```

### Secure Fields

For sensitive data like passwords:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
# use waterui::form::{secure, SecureField};
# use waterui::form::secure::Secure;

pub fn password_form() -> impl View {
    let password: Binding<Secure> = binding(Secure::default());
    let confirm_password: Binding<Secure> = binding(Secure::default());

    vstack((
        secure("Password:", &password),
        secure("Confirm Password:", &confirm_password),
        password_validation(&password, &confirm_password),
    ))
}

fn password_validation(pwd: &Binding<Secure>, confirm: &Binding<Secure>) -> impl View {
    let feedback = pwd.clone().zip(confirm.clone()).map(|(p, c)| {
        if p.expose() == c.expose() && !p.expose().is_empty() {
            "Passwords match ✓".to_string()
        } else {
            "Passwords do not match".to_string()
        }
    });

    text(feedback)
}
```

# Form Validation Best Practices

### Real-time Validation with Computed Signals

For more complex forms, it's a good practice to encapsulate your validation logic into a separate struct. This makes your code more organized and reusable.

Let's create a `Validation` struct that holds computed signals for each validation rule.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
# use waterui::SignalExt;
# use waterui::widget::condition::when;
# use waterui_form::{form, FormBuilder};
#[derive(Default, Clone, Debug, waterui_derive::Project, waterui_derive::FormBuilder)]
struct ValidatedForm {
    email: String,
    password: String,
    age: i32,
}

pub fn validated_form_view() -> impl View {
    let form_state = ValidatedForm::binding();
    let is_valid_email = form_state.clone().map(|f| f.email.contains('@') && f.email.contains('.'));
    let is_valid_password = form_state.clone().map(|f| f.password.len() >= 8);
    let is_valid_age = form_state.clone().map(|f| f.age >= 18);

    let can_submit = is_valid_email
        .clone()
        .zip(is_valid_password.clone())
        .zip(is_valid_age.clone())
        .map(|((email, password), age)| email && password && age);

    let email_feedback = is_valid_email.clone().map(|valid| {
        if valid {
            "✓ Valid email".to_string()
        } else {
            "✗ Please enter a valid email".to_string()
        }
    });
    let password_feedback = is_valid_password.clone().map(|valid| {
        if valid {
            "✓ Password is strong enough".to_string()
        } else {
            "✗ Password must be at least 8 characters".to_string()
        }
    });
    let age_feedback = is_valid_age.clone().map(|valid| {
        if valid {
            "✓ Age requirement met".to_string()
        } else {
            "✗ Must be 18 or older".to_string()
        }
    });

    let submit_binding = form_state.clone();
    let form_binding = form_state.clone();

    vstack((
        form(&form_binding),
        text(email_feedback),
        text(password_feedback),
        text(age_feedback),
        when(can_submit.clone(), move || {
            button("Submit").action_with(&submit_binding, |state: Binding<ValidatedForm>| {
                println!("Form submitted: {:?}", state.get());
            })
        })
        .or(|| text("Fill every requirement to enable submission.")),
    ))
}
```

## Integration with State Management

Forms integrate seamlessly with WaterUI's reactive state system:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::SignalExt;
# use waterui::widget::condition::when;
# use waterui::Binding;
# use waterui_form::{form, FormBuilder};
#[derive(Default, Clone, Debug, waterui_derive::Project, waterui_derive::FormBuilder)]
struct UserSettings {
    name: String,
    theme: String,
    notifications: bool,
}

pub fn settings_panel() -> impl View {
    let settings = UserSettings::binding();
    let has_changes = settings.clone().map(|s| {
        s.name != "Default Name" || s.theme != "Light" || s.notifications
    });

    let settings_summary = settings.clone().map(|s| {
        format!(
            "User: {} | Theme: {} | Notifications: {}",
            s.name,
            s.theme,
            if s.notifications { "On" } else { "Off" }
        )
    });
    let form_binding = settings.clone();

    vstack((
        "Settings",
        form(&form_binding),
        "Preview:",
        text(settings_summary),
        when(has_changes.clone(), {
            let save_binding = settings.clone();
            move || {
                button("Save Changes").action_with(&save_binding, |state: Binding<UserSettings>| {
                    save_settings(&state.get());
                })
            }
        })
        .or(|| text("No changes to save.")),
    ))
}

fn save_settings(settings: &UserSettings) {
    println!("Saving settings: {settings:?}");
}
```
