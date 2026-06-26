#!/usr/bin/env swift
//
//  generate-help-topics.swift
//
//  Generates the `HelpTopic` enum used by `.helpInfo(...)` from an mdBook table
//  of contents (`SUMMARY.md`). One enum case is emitted per documentation page,
//  with the page's anchor as the raw value and the page's title (the link text
//  in SUMMARY.md) as `helpTitle`.
//
//  This keeps the set of help anchors in lockstep with the actual book:
//  referencing a page that no longer exists becomes a compile error, and new
//  pages show up as new cases automatically.
//
//  Usage:
//      swift generate-help-topics.swift <SUMMARY.md> <output.swift>
//
//  Typically run from a "Run Script" build phase (see HelpInfo/README), but it's
//  safe to run by hand, too.
// See https://codeberg.org/coffee_nebula/mdbook-applehelp
// for the mdbook backend which creates a .help bundle

import Foundation

let arguments = CommandLine.arguments
guard arguments.count >= 3 else {
    FileHandle.standardError.write(Data(
        "usage: generate-help-topics.swift <SUMMARY.md> <output.swift>\n".utf8))
    exit(2)
}

let summaryPath = arguments[1]
let outputPath = arguments[2]

let summary: String
do {
    summary = try String(contentsOfFile: summaryPath, encoding: .utf8)
} catch {
    FileHandle.standardError.write(Data("error: cannot read \(summaryPath): \(error)\n".utf8))
    exit(1)
}

// Matches Markdown links: [Title](path). We only keep ones whose target is a
// local `.md` file (the chapters); section headers and external links are
// ignored.
let linkRegex = try! NSRegularExpression(pattern: #"\[([^\]]+)\]\(([^)]+)\)"#)

struct Topic {
    let caseName: String
    let anchor: String
    let title: String
}

/// Turns a page anchor like `getting-started/main-window` into a lowerCamelCase
/// Swift identifier like `gettingStartedMainWindow`.
func caseName(for anchor: String) -> String {
    let parts = anchor
        .split(whereSeparator: { $0 == "/" || $0 == "-" || $0 == "_" })
        .map(String.init)
    guard let first = parts.first else { return anchor }
    let rest = parts.dropFirst().map { $0.prefix(1).uppercased() + $0.dropFirst() }
    return ([first] + rest).joined()
}

func swiftStringLiteral(_ raw: String) -> String {
    raw.replacingOccurrences(of: "\\", with: "\\\\")
       .replacingOccurrences(of: "\"", with: "\\\"")
}

var topics: [Topic] = []
var seenAnchors = Set<String>()
var seenCaseNames = Set<String>()

for line in summary.components(separatedBy: .newlines) {
    let ns = line as NSString
    let matches = linkRegex.matches(in: line, range: NSRange(location: 0, length: ns.length))
    for match in matches {
        let title = ns.substring(with: match.range(at: 1))
        var path = ns.substring(with: match.range(at: 2))

        guard path.hasSuffix(".md") else { continue }
        if path.hasPrefix("./") { path.removeFirst(2) }
        let anchor = String(path.dropLast(3)) // strip ".md"
        guard !seenAnchors.contains(anchor) else { continue }
        seenAnchors.insert(anchor)

        var name = caseName(for: anchor)
        // Defend against the unlikely event of two pages collapsing to the same
        // identifier — suffix with a counter so the file still compiles.
        if seenCaseNames.contains(name) {
            var n = 2
            while seenCaseNames.contains("\(name)\(n)") { n += 1 }
            name = "\(name)\(n)"
        }
        seenCaseNames.insert(name)

        topics.append(Topic(caseName: name, anchor: anchor, title: title))
    }
}

var out = """
// Generated from the mdBook SUMMARY.md — do not edit by hand.
// Re-run the help-topics generator to refresh it.

import SwiftUI

/// Every documentation page in the app's Apple Help book — one case per chapter,
/// generated from `SUMMARY.md`.
///
/// The raw value is the page anchor passed to `NSHelpManager.openHelpAnchor`;
/// `helpTitle` is the chapter title from the table of contents. Because this
/// type is regenerated from the book on every build, referring to a page that
/// doesn't exist is a compile error.
public enum HelpTopic: String, CaseIterable, HelpTopicRepresentable {

"""

for topic in topics {
    out += "    case \(topic.caseName) = \"\(topic.anchor)\"\n"
}

out += "\n    public var helpAnchor: String { rawValue }\n\n"
out += "    public var helpTitle: String {\n        switch self {\n"
for topic in topics {
    out += "        case .\(topic.caseName): return \"\(swiftStringLiteral(topic.title))\"\n"
}
out += "        }\n    }\n}\n\n"

out += """
public extension View {
    /// Type-safe sugar for `.helpInfo(_:alignment:padding:)`:
    /// `.helpInfo(.voicesCloning, .voicesDesign)`.
    func helpInfo(_ topics: HelpTopic...,
                  alignment: Alignment = .bottomTrailing,
                  padding: CGFloat = 12) -> some View {
        helpInfo(topics.map { $0 as any HelpTopicRepresentable },
                 alignment: alignment,
                 padding: padding)
    }
}

"""

do {
    try out.write(toFile: outputPath, atomically: true, encoding: .utf8)
    FileHandle.standardError.write(Data(
        "Generated \(topics.count) help topics → \(outputPath)\n".utf8))
} catch {
    FileHandle.standardError.write(Data("error: cannot write \(outputPath): \(error)\n".utf8))
    exit(1)
}
