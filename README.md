# mdbook-applehelp

This backend along with [this build script](generate-help-topics.swift) allow you to write and maintain docs for your app as an mdBook, and output a .help bundle (html, css, assets, spotlight index) that gets imbedded in your .app

For best use, set up build steps to:
* Build the .help bundle (mdbook build)
* Run generate-help-topics.swift on your SUMMARY.md, which will output a .swift file containing an enum of all your chapters.
This is used to have compile-time checks of help references: rename or remove a chapter in the book 
and you get compile errors from code that refered to the old references.


[Authoring Apple Help](https://developer.apple.com/library/archive/documentation/Carbon/Conceptual/ProvidingUserAssitAppleHelp/authoring_help/authoring_help_book.html)
