## ADDED Requirements

### Requirement: Loader visible on initial page render

The HTML entry point SHALL render loader content that is visible during the initial document paint, before the WASM bundle has finished fetching or instantiating. The loader SHALL be expressed using only inline HTML and CSS in `web-client/index.html` (no images, no external font dependencies, no external scripts) so that it remains legible even when subresources are still in flight or unreachable.

#### Scenario: WASM fetch in progress
- **WHEN** the browser has parsed `index.html` but the WASM bundle download has not yet completed
- **THEN** the page displays the loader (text + spinner) instead of a blank canvas

#### Scenario: subresources unavailable
- **WHEN** only the HTML document has been retrieved and other subresources (fonts, additional CSS) are blocked or pending
- **THEN** the loader text and spinner are still rendered legibly

### Requirement: Loader dismissal on application start

When the WASM application signals successful boot by dispatching the `TrunkApplicationStarted` event on `window`, the loader SHALL be hidden or removed so that it does not visually obscure the application canvas.

#### Scenario: normal boot
- **WHEN** `TrunkApplicationStarted` is dispatched on `window`
- **THEN** the loader is no longer visible to the user, and the canvas occupies its intended area

#### Scenario: dismissal preserves canvas interactivity
- **WHEN** the loader has been dismissed
- **THEN** no remnant of the loader element intercepts pointer events that should reach the canvas

### Requirement: Loader timeout error state

If `TrunkApplicationStarted` has not fired within a configured timeout (target: ~30 seconds from page load), the loader SHALL transition to a visible error state that informs the user that boot is taking longer than expected and suggests refreshing the page. The transition SHALL NOT remove the loader prematurely; it only swaps the content shown inside it.

#### Scenario: boot exceeds timeout
- **WHEN** the configured timeout has elapsed since page load and `TrunkApplicationStarted` has not been observed
- **THEN** the loader content swaps to a message indicating boot is taking longer than expected and a suggestion to refresh

#### Scenario: late boot after timeout
- **WHEN** the timeout has already fired and the error state is visible, and then `TrunkApplicationStarted` is dispatched
- **THEN** the error state is removed and the application is allowed to take over the canvas (the late event still dismisses the loader)
