# TypeScript views

> **In this chapter, you will:**
> - Author a view in TSX over the same component catalog the Rust API uses, and mount it with `tsx!`
> - Declare a props contract with `#[derive(TsProps)]` and know how Rust types map to TypeScript
> - Use signals, `<Show>`, `<For>`, `<Suspense>`, `<Box>`, contexts, the theme and the locale
> - Read modifier attribute order the way you read a Rust modifier chain
> - Know which engine runs your code on each platform, and how a test loads your bundle

Some views change more often than the binary that hosts them, or are written by people who live in TypeScript. WaterUI lets you author such a view in TSX and mount it from Rust. Nothing is drawn by a web view: the module runs in an embedded JavaScript engine, and every element it creates is a real WaterUI view, so a `<Text>` is the same `Text` you would write in Rust.

There is no virtual tree and no diffing pass. An element evaluates once into a native view handle. Reactivity stays fine-grained on both sides of the bridge: a signal write in TypeScript updates exactly the view that reads it, and a `Binding` the Rust side owns reaches TypeScript as a signal. If you have read [the reactive chapter](../02-core/02-reactive.md), the model is the one you already know.

## Enabling the feature

TypeScript views are opt-in. Enable the `ts` feature on the `waterui` facade in your application crate:

```toml
[dependencies]
waterui = { version = "*", features = ["ts"] }
```

Replace the version requirement with the release you build against. The `ts-ota` feature implies `ts` and adds the over-the-air client, covered at the end of this chapter. Without `ts`, none of this chapter's code is linked.

## A first module

A module is an ordinary `.tsx` file whose default export is a function from props to an element. Components import from `"waterui"`:

```tsx
import { Button, Text, VStack } from "waterui";
import type { Signal } from "waterui";

export interface PromoProps {
  headline: string;
  unread: Signal<number>;
  onDismiss: () => void;
}

export default function Promo({ headline, unread, onDismiss }: PromoProps) {
  return (
    <VStack spacing={8}>
      <Text>{headline}</Text>
      <Text>{() => `${unread()} unread`}</Text>
      <Button onTap={onDismiss}>Dismiss</Button>
    </VStack>
  );
}
```

`{headline}` is read once. `{() => ...}` is an accessor, so that `<Text>` re-renders its content whenever `unread` changes and nothing else does. A `<Text>` whose only child is a string treats it as a translation key, even when the string came from Rust; [Localization](#localization) below has the rules.

The vocabulary is the component catalog the facade publishes: `VStack`, `HStack`, `ZStack`, `ScrollView`, `Spacer`, `Divider`, `Text`, `Label`, `Button`, `Toggle`, `Slider`, `Stepper`, `TextField`, `Picker`, `List`, `Progress`, `Link`, `Card`, `Badge`, `Table`, `Column`, `Accordion`, `Avatar`, `NavigationStack`, `NavigationLink`, `Tabs` and `Tab`, plus `Image` when the facade's `media` feature is on. A tag the catalog does not carry is a typed error at mount, never a silent fallback. A component whose content is a builder, such as a navigation destination, takes a render function typed `() => JSX.Element`.

## Mounting from Rust

You mount a module where you use it, with `tsx!`. Its props are a struct literal, and the struct derives `TsProps`:

```rust,ignore
use waterui::prelude::*;
use waterui::ts::schema::TsProps;
use waterui::tsx;

#[derive(TsProps)]
struct PromoProps {
    headline: String,
    unread: Binding<u32>,
    #[ts(rename = "onDismiss")]
    on_dismiss: Box<dyn Fn()>,
}

fn promo(unread: Binding<u32>, dismissed: Binding<bool>) -> impl View {
    tsx!(
        "./promo.tsx",
        PromoProps {
            headline: String::from("Welcome back"),
            unread,
            on_dismiss: Box::new(move || dismissed.set(true)),
        }
    )
}
```

The fence is not compiled with the book: it needs the `ts` feature and a real `promo.tsx` beside the Rust file.

`tsx!` returns an ordinary view, so it slots anywhere a view goes: inside a `vstack`, under a modifier, as a navigation destination.

### The path and the module id

The path is relative to the Rust file that writes the `tsx!` call, the way `include_str!` resolves its argument. The macro stats the file at compile time, so a missing file is a compile error naming the path, and it never parses the file. An absolute path, or a file outside the crate, is rejected.

The **module id** is the file's path relative to the crate root (`CARGO_MANIFEST_DIR`), with forward slashes and the extension kept. `tsx!("./promo.tsx", ...)` written in `src/lib.rs` mounts the module `src/promo.tsx`. The bundle publishes modules under those ids, and a mount failure names the id.

A module that takes no props is mounted with `tsx!("./about.tsx")`. It is typed against `waterui::ts::NoProps`, the empty contract, and is checked like any other.

### Failure at mount

`Mount`, the type `tsx!` expands to, implements `View`, and `View::body` has no error channel. When the module cannot be mounted, `body` panics with a message naming the module and the cause: no runtime installed, a module the bundle does not carry, a contract mismatch, or a module that threw while building its tree. Every one of these means the bundle and the binary disagree, which is a build-time mistake. A caller who wants to decide for itself calls `Mount::try_mount(self, &Environment)`, which returns `Result<AnyView, TsError>`. `Mount` and `TsError` are in `waterui::ts`.

## The props contract

A `.tsx` module and the Rust struct that mounts it must agree on the shape of the props. `#[derive(TsProps)]` makes that agreement checkable: it derives a **contract hash** from the struct's schema, and the bundle records the hash of the interface each module was built against. At mount, the two are compared. A bundle built against a different contract fails with an error naming both hashes, instead of handing the module an object of a shape it was not compiled for.

Here is how field types project into TypeScript:

| Rust field type | TypeScript prop |
| --- | --- |
| `String`, `&'static str`, `Str` | `string` |
| `bool` | `boolean` |
| `f32`, `f64`, `i8`..`i32`, `u8`..`u32` | `number` |
| `i64`, `u64`, `isize`, `usize` | `bigint` |
| `Option<T>` | `T \| null` |
| `Vec<T>`, `&'static [T]` | `T[]` |
| `[T; N]` | a fixed-length tuple, `[T, T, ...]` |
| `BTreeMap<K, V>`, `HashMap<K, V>` with a string key | `Record<string, V>` |
| `Binding<T>` | `Signal<T>`, readable and writable |
| `Computed<T>` | `Accessor<T>`, read-only |
| `Box<dyn Fn(A, B)>`, `Arc<dyn Fn(A, B)>` (with or without `Send`/`Sync`), `Rc<dyn Fn(A, B)>`, `fn(A, B)`, up to eight arguments | `(arg0: A, arg1: B) => void` |
| `AnyView` | `View` |

Integers wider than a `number` can hold exactly become `bigint`. Callbacks cross as functions that return nothing, and each argument type must be readable back out of JavaScript, because TypeScript is what calls them.

`#[ts(rename = "onDismiss")]` maps a snake_case Rust field to the camelCase prop TypeScript writes. The rename travels through the schema and the conversions together, so one declaration covers both.

Types nested inside a props struct derive `TsType` rather than `TsProps`: a struct becomes an object type, an enum of unit variants becomes a union of string literals, and an enum carrying data becomes a tagged object. Only the root props struct derives `TsProps`, which also records the contract for the build tooling. A nested type that carries a value that only travels outward, such as a callback, declares `#[ts(one_way)]`.

Two practical consequences follow. A binary can hold one props type of a given name, because the contract record is named after the type. And because the contract is checked by hash, changing a props struct without rebuilding the bundle is caught when the bundle is loaded or the module mounted, instead of misbehaving later.

## Attribute order is semantic

Modifier attributes apply in the order you write them, left to right, exactly like a Rust modifier chain. These two elements carry the same attributes and are two different views:

```tsx
// text("Attribute order").padding_with(16.0).background(color)
<Text padding={16} background={accent}>Attribute order</Text>

// text("Attribute order").background(color).padding_with(16.0)
<Text background={accent} padding={16}>Attribute order</Text>
```

The first insets the text and then paints the color behind the inset, so the color covers the padding. The second paints the color behind the text and then insets, so the color hugs the text and the padding is transparent. [The modifiers chapter](../02-core/04-modifiers.md) covers why the same is true in Rust.

The modifiers the catalog accepts are `padding`, `background`, `foreground`, `opacity`, `shadow`, `border`, `clip`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `disabled`, `onTapGesture`, `a11yLabel`, `a11yValue`, `a11yId` and `a11yHidden`.

Because order is semantic, a spread may not carry a modifier attribute. `{...rest}` would hide where `padding` sits relative to `background`. The transform rejects it statically, and the runtime throws as a backstop. Configuration attributes such as `spacing={8}` or `onTap={...}` configure the component itself, are not ordered, and may arrive through a spread.

## Signals

TypeScript reactivity follows the Rust model. `Signal<T>` has the shape of `Binding<T>`, and `Accessor<T>` has the shape of `Computed<T>`:

```tsx
import { createMemo, createSignal } from "waterui";

const count = createSignal(0);
count();                         // read, tracked
count.set(1);                    // write
count.update((v) => v + 1);

const doubled = createMemo(() => count() * 2);   // derived, read-only
const [read, write] = createSignal(10);          // destructuring works too
```

Props are getter props. Writing `padding={count()}` in JSX is the live form: the transform emits a getter, so a component that reads `props.padding` reads the current value, never a snapshot taken at setup. Anywhere a value can be dynamic, the position accepts a constant, a signal or an accessor.

State that flows back, such as a toggle's value, a slider's position or a text field's contents, takes a writable `Signal`:

```tsx
const done = createSignal(false);

<Toggle value={done}>Mark complete</Toggle>
```

On the Rust side that prop is a `Binding<bool>`, and writes in either language land in the same cell. The same primitives you may know from Solid are available from `"waterui"`: `createEffect`, `createRoot`, `batch`, `untrack`, `onCleanup`, `createStore`, `isSignal` and `isAccessor`. Ownership is explicit and not positional, so there are no hook rules: a primitive belongs to the reactive scope that is active when you create it.

## Control flow

Four control-flow components ship with the library. Each branch callback runs under a fresh reactive scope, so a branch that is disposed takes its signals and `onCleanup` callbacks with it.

`<Show>` presents its children while `when` is truthy and `fallback` otherwise. A function child is a render prop receiving an accessor of the current truthy value:

```tsx
<Show when={loggedIn} fallback={<Text>Sign in</Text>}>
  <Text>Welcome back</Text>
</Show>

<Show when={selected}>{(item) => <Text>{() => item().name}</Text>}</Show>
```

`<For>` is keyed reconciliation over a collection. `each` is the list, `by` picks the key (the item's own identity when omitted), and the child is a render function receiving the item and an accessor of its index:

```tsx
<For each={items} by={(item) => item.id}>
  {(item, index) => <Text>{item.name}</Text>}
</For>
```

Keys must be unique and stable. A `by` that returns the same key for two rows, or a key the host cannot hold, is an error at mount and a panic on a later update, not a silent mis-render.

`<Suspense fallback={...}>` is the host-driven pending state: the fallback shows while the children are pending. It is the TypeScript counterpart of [the suspense chapter](03-suspense.md).

### `<Box>`: a modifier scope

`<Box>` creates no node. It takes exactly one element child and applies its modifier attributes to that child in written order. That makes it the neutral container for putting modifiers on a subtree, and for writing one modifier twice, which JSX's duplicate-attribute rule otherwise forbids:

```tsx
<Box padding={8}>
  <Box padding={16} background={accent}>
    <Text>Framed</Text>
  </Box>
</Box>
```

`<Box>` accepts only modifier attributes. A configuration attribute or a non-element child is an error.

## Contexts, theme and locale

`createContext` and `useContext` are the ordinary context pair, resolved through the owner chain. A context's `Provider` sets the value for everything its subtree creates, and `useContext` outside any provider answers the default:

```tsx
import { createContext, useContext } from "waterui";

const Density = createContext("comfortable");

function Row() {
  const density = useContext(Density);
  return <Text>{density}</Text>;
}

<Density.Provider value="compact">
  <Row />
</Density.Provider>
```

Two framework contexts are seeded at mount from the host's environment, and both are read-only accessors:

```tsx
import { useLocale, useTheme } from "waterui";

const theme = useTheme();   // Accessor<{ colorScheme: "light" | "dark", ...tokens }>
const locale = useLocale(); // Accessor<{ identifier, languageCode, textDirection }>

<Box background={theme().surface}>
  <Text>{() => locale().identifier}</Text>
</Box>
```

`useTheme()` answers the color scheme and every color token the environment installs (`surface`, `accent`, `foreground` and so on). A token that no theme installs is absent rather than defaulted. `useLocale()` answers `identifier`, the canonical BCP-47 tag such as `"en-US"`; `languageCode`, the language subtag alone such as `"en"`; and `textDirection`, `"ltr"` or `"rtl"`. Both throw outside a mounted tree. For the Rust side of the environment, see [the environment chapter](../02-core/03-environment.md).

### Localization

`<Text>` localizes through the same `TranslationCatalog` as Rust's `text("...")`, so one catalog serves both languages of a codebase:

```tsx
<Text>Settings</Text>          // looked up in the catalog for the current locale
<Text>{count()}</Text>         // a number is the value itself, never a key
```

A single string child is a translation key, and a key the catalog does not carry renders as the key itself, exactly like `Text::localized`. A number child is formatted verbatim and never looked up. Text assembled from several children, such as `<Text>Hello {name}</Text>`, is joined and shown verbatim, never looked up: only a string you write as the one child is a key. This differs from Rust's `text!("Hello {name}")`, whose format string is itself a translation key. Catalogs and locales are covered in [the i18n chapter](06-i18n.md).

## Bundling

At run time the engine does not read your `.tsx` files. Your modules are compiled into one **bundle**: a single classic script holding the WaterUI JavaScript library plus every module the binary mounts, with a manifest that records, for each module, the contract hash it was built against.

The `water` CLI is the intended bundler. The `tsx!` macro and the `TsProps` derive each leave a record in the compiled artifact (`waterui_meta_tsx_*` and `waterui_meta_tsprops_*`, in debug builds), the same channel the rest of the toolchain uses to learn about your code. The CLI reads those records to learn which modules the binary mounts and which contract each must satisfy, and bundles exactly those modules. It never scans your source for `tsx!`.

The bundle is only valid for the runtime it was built against. Its manifest carries a runtime fingerprint, made from a hash of the embedded JavaScript library and a hash of the component catalog, and the loader refuses a bundle whose fingerprint or module contracts do not match the binary.

In an application, the runtime every `tsx!` mount needs is installed at launch. The bundle built into the binary is the **baseline**, and code the CLI generates hands it to `waterui::ts::Loader`, which verifies it against the binary and returns the environment your root view renders under, with the runtime and the bundle's translations installed. You do not write that code yourself.

> **Note.** The `water` CLI at the version this book covers does not yet ship the TSX transform, the bundler or the `water ota` commands. The macros, the records, the manifest format and the loader are in the framework, but there is no supported command that produces a bundle today. Check `water --help` on your installed version.

## Engines and platform support

The JavaScript engine is chosen by target:

| Target | Engine |
| --- | --- |
| iOS, macOS, tvOS, visionOS | the system JavaScriptCore, which adds no binary size |
| Android, Linux, Windows | QuickJS-NG, embedded |
| watchOS | unsupported |

watchOS ships no JavaScriptCore, and embedding QuickJS-NG is not supported there either, so the runtime crate is a compile error on watchOS. That is a deliberate asymmetry, not a missing port. Platform tiers for the rest of WaterUI are in [the platforms chapter](../01-getting-started/05-platforms.md).

## Testing a mounted module

A `tsx!` view is tested like any other, with `#[waterui::test]` over the mounted tree. A test session has no bundle loader, so it takes the bundle from the `WATERUI_TS_BUNDLE` environment variable instead. Enable the `ts` feature of `waterui-testing` for this:

```toml
[dev-dependencies]
waterui-testing = { version = "*", features = ["ts"] }
```

With the feature on, a session started with `waterui_testing::ui()` loads the bundle the variable names and installs its runtime for the view it mounts. `mount_app` runs your application's own environment instead, where the application's loader installs the runtime, and does not read the variable.

The path rules are strict:

- An absolute path is used as written.
- A relative path is resolved against `CARGO_MANIFEST_DIR`, which cargo and nextest set for every test binary. Without that variable a relative path is an error, never a path taken from the working directory.
- A variable that is set but empty fails the session, and so does one naming a file that cannot be read or loaded, with an error naming the path.
- When the variable is unset, nothing fails until a view reaches a `tsx!` mount, which then fails with a message naming the variable. A test that mounts no TypeScript never needs it.

The framework's messages name `water test` and `water preview` as the commands that build the bundle and set the variable. Neither does so at the version this book covers: the CLI has no `test` command, and no preview host reads the variable yet, so a `#[preview]` of a `tsx!` view fails at the mount. Until the bundler ships, set `WATERUI_TS_BUNDLE` yourself and run `cargo nextest run`. See [the testing chapter](../07-tools/03-testing.md) for the rest of the testing workflow.

## Over-the-air bundles

With the `ts-ota` feature, an application can receive a newer bundle without a new binary. The client fetches a signed manifest and the bundle it names, verifies both, and caches them for the next launch. It fetches the application's own view bundle and nothing else.

The pieces, in the order the loader uses them:

- The **baseline** is the bundle built into the binary. It is the floor: a launch can always fall back to it, and it is never marked bad.
- A downloaded bundle must pass, in order, an ed25519 signature check under the key embedded in the binary, the runtime fingerprint, the contract hash of every module the binary mounts, a check that its translations parse, a size bound taken from the signed manifest, and a SHA-256 digest of the bundle bytes. Nothing is cached until every check passes, and a bundle is downloaded only when its version is newer than the baseline's.
- Selection happens at cold start only. There is no hot swap: cached versions are tried newest first, skipping those marked bad. A bundle that throws while it is evaluated is marked bad and the next one is tried in the same launch. A launch that dies before the application reports its first frame, for example because a mount panicked, marks that version bad at the next launch, so a broken update cannot cause a crash loop. With nothing usable, the baseline runs.
- A bundle may also carry translations, which then shadow the application's compiled-in catalog.

Because the contract hash is part of verification, an over-the-air update can change what a module does and how it looks, but not the props the Rust side passes it. Changing a props struct needs a new binary.

The full protocol, including the manifest format and the store layout, is specified in `OTA.md` beside the `waterui-ts` crate in the framework repository.

## Next steps

Try porting one small screen of an existing app: write its props struct and the matching TypeScript interface, and mount the module with `tsx!`. The Rust side, the derive and the mount site compile today; the module runs once a bundle for it is loaded. Then read [the suspense chapter](03-suspense.md) for the Rust side of async views, or [the testing chapter](../07-tools/03-testing.md) for how mounted trees are asserted.
