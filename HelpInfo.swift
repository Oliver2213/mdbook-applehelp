//  HelpInfo.swift
//
//  A small, self-contained way to attach a standard macOS Help (`?`) button to
//  any SwiftUI view and have it open a page in the app's Apple Help book.
//
//  This file has **no dependency on app internals**. The single-topic case uses
//  SwiftUI's native `HelpLink(anchor:)`, which opens an anchor in the app's
//  registered help book (the one named by `CFBundleHelpBookName`). The only
//  app-specific piece is the list of topics, supplied by a generated
//  `HelpTopic` enum (see `HelpTopic+Generated.swift`) — keeping this file
//  reusable verbatim.
//
//  Requires macOS 14+ (for `HelpLink`).
//
//  Usage:
//
//      SomeView()
//          .helpInfo(.someTopic)                       // single topic
//          .helpInfo(.someTopic, .otherTopic)        // menu of topics
//
//  Because the topic list is generated from the book's `SUMMARY.md`, referring
//  to a page that doesn't exist is a compile error.

import SwiftUI
import AppKit // NSHelpManager — used only to open a chosen topic in the multi-topic menu.

// MARK: - Topic protocol

/// A documentation page that can be opened in the app's Apple Help book.
///
/// Conformers expose a stable `helpAnchor` — the page's anchor inside the help
/// bundle, e.g. `"voices/cloning"`, which is what `mdbook-applehelp` emits as
/// `<a name="voices/cloning">` and what Apple Help expects — and a
/// human-readable `helpTitle` used to label the entry when more than one topic
/// is offered in a menu.
///
/// The generated `HelpTopic` enum is the canonical conformer; you should not
/// need to implement this yourself.
public protocol HelpTopicRepresentable {
    /// The page anchor inside the help book (path of the chapter, no extension).
    var helpAnchor: String { get }
    /// The page's title, shown as the menu label when several topics are offered.
    var helpTitle: String { get }
}

// MARK: - View modifier

public extension View {
    /// Overlays a standard macOS Help (`?`) button on this view that opens the
    /// app's Apple Help book.
    ///
    /// - One topic: clicking the button jumps straight to that page.
    /// - Several topics: clicking presents a menu of titled topics.
    ///
    /// The button uses SwiftUI's native `HelpLink`, so it always matches the
    /// system affordance and opens the app's registered help book
    /// (`CFBundleHelpBookName`). Pass an empty array to render nothing.
    ///
    /// - Parameters:
    ///   - topics: The page(s) the button can open.
    ///   - alignment: Where to place the button within this view's bounds.
    ///     Defaults to `.bottomTrailing`.
    ///   - padding: Inset from the chosen corner, in points. Defaults to `12`.
    func helpInfo(_ topics: [any HelpTopicRepresentable],
                  alignment: Alignment = .bottomTrailing,
                  padding: CGFloat = 12) -> some View {
        overlay(alignment: alignment) {
            HelpInfoButton(topics: topics)
                .padding(padding)
        }
    }
}

// MARK: - Book name resolution

/// The Apple Help book name for the running app: the main bundle's identifier
/// with `.help` appended, matching `CFBundleHelpBookName`. Returns `nil` if the
/// app has no bundle identifier (e.g. some test hosts).
///
/// `HelpLink(anchor:)` finds the book on its own; this is only needed to open a
/// topic chosen from the multi-topic menu, where we go through `NSHelpManager`.
func resolvedHelpBookName() -> String? {
    guard let identifier = Bundle.main.bundleIdentifier else { return nil }
    return identifier + ".help"
}

// MARK: - Button view

/// Renders the help button: a native `HelpLink` that opens the topic directly
/// when there's one, or offers a menu of titles when there are several. Kept
/// private — callers use `.helpInfo(...)`.
private struct HelpInfoButton: View {
    let topics: [any HelpTopicRepresentable]

    @State private var isMenuPresented = false

    var body: some View {
        switch topics.count {
        case 0:
            EmptyView()
        case 1:
            // Native path: HelpLink opens the anchor in the app's help book.
            HelpLink(anchor: topics[0].helpAnchor)
        default:
            HelpLink { isMenuPresented = true }
                .confirmationDialog("Help Topics",
                                    isPresented: $isMenuPresented,
                                    titleVisibility: .visible) {
                    ForEach(Array(topics.enumerated()), id: \.offset) { _, topic in
                        Button(topic.helpTitle) { open(topic) }
                    }
                }
        }
    }

    private func open(_ topic: any HelpTopicRepresentable) {
        guard let book = resolvedHelpBookName() else { return }
        NSHelpManager.shared.openHelpAnchor(topic.helpAnchor, inBook: book)
    }
}
