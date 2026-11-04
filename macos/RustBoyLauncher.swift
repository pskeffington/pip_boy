import AppKit
import Foundation

@MainActor
final class RustBoyAppDelegate: NSObject, NSApplicationDelegate {
    private var statusItem: NSStatusItem!
    private var server: Process?
    private var dashboardURL: URL?
    private var pendingPath = "/"
    private var outputBuffer = ""

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        statusItem.button?.image = NSImage(
            systemSymbolName: "dot.radiowaves.left.and.right",
            accessibilityDescription: "Rust-Boy"
        )
        statusItem.button?.toolTip = "Rust-Boy local workstation"
        rebuildMenu()
        startServer()
    }

    func applicationWillTerminate(_ notification: Notification) {
        stopServer()
    }

    @objc private func openDashboard() { open(path: "/") }
    @objc private func openMicroHUD() { open(path: "/micro") }
    @objc private func stopDashboard() { stopServer(); rebuildMenu() }
    @objc private func quit() { NSApp.terminate(nil) }

    private func open(path: String) {
        pendingPath = path
        guard let dashboardURL else {
            startServer()
            return
        }
        if let target = URL(string: path, relativeTo: dashboardURL)?.absoluteURL {
            NSWorkspace.shared.open(target)
        }
    }

    private func startServer() {
        guard server == nil else { return }
        let executable = Bundle.main.bundleURL
            .appendingPathComponent("Contents/MacOS/pipboy", isDirectory: false)
        let process = Process()
        process.executableURL = executable
        process.arguments = ["dashboard", "--port", "0"]
        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors
        output.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty, let text = String(data: data, encoding: .utf8) else { return }
            DispatchQueue.main.async { self?.consumeOutput(text) }
        }
        errors.fileHandleForReading.readabilityHandler = { handle in
            if handle.availableData.isEmpty { handle.readabilityHandler = nil }
        }
        process.terminationHandler = { [weak self] _ in
            DispatchQueue.main.async {
                guard self?.server === process else { return }
                self?.server = nil
                self?.dashboardURL = nil
                self?.rebuildMenu()
            }
        }
        do {
            try process.run()
            server = process
            rebuildMenu()
        } catch {
            server = nil
            NSLog("Rust-Boy dashboard failed to start: %@", error.localizedDescription)
        }
    }

    private func consumeOutput(_ text: String) {
        outputBuffer += text
        let lines = outputBuffer.components(separatedBy: "\n")
        outputBuffer = lines.last ?? ""
        guard let line = lines.dropLast().first(where: { $0.hasPrefix("Open http://127.0.0.1:") }),
              let url = URL(string: String(line.dropFirst("Open ".count))) else {
            return
        }
        dashboardURL = url
        rebuildMenu()
        if let readyFile = ProcessInfo.processInfo.environment["RUSTBOY_READY_FILE"] {
            let record = "\(url.absoluteString)\n\(server?.processIdentifier ?? 0)\n"
            try? record.write(toFile: readyFile, atomically: true, encoding: .utf8)
        }
        if ProcessInfo.processInfo.environment["RUSTBOY_DISABLE_BROWSER_OPEN"] == "1" {
            return
        }
        if let target = URL(string: pendingPath, relativeTo: url) {
            NSWorkspace.shared.open(target)
        }
    }

    private func stopServer() {
        if let server, server.isRunning { server.terminate() }
        server = nil
        dashboardURL = nil
    }

    private func rebuildMenu() {
        let menu = NSMenu()
        let title = dashboardURL == nil ? "Starting local dashboard…" : "Dashboard running on this Mac"
        let state = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        state.isEnabled = false
        menu.addItem(state)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Open Desktop Dashboard", action: #selector(openDashboard), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Open Field HUD", action: #selector(openMicroHUD), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Stop Dashboard", action: #selector(stopDashboard), keyEquivalent: ""))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit Rust-Boy", action: #selector(quit), keyEquivalent: "q"))
        for item in menu.items where item.action != nil { item.target = self }
        statusItem.menu = menu
    }
}

@main
struct RustBoyLauncher {
    @MainActor
    static func main() {
        if ProcessInfo.processInfo.environment["RUSTBOY_HEADLESS_SMOKE"] == "1" {
            runHeadlessSmoke()
            return
        }
        let app = NSApplication.shared
        let delegate = RustBoyAppDelegate()
        app.delegate = delegate
        app.run()
    }

    private static func runHeadlessSmoke() {
        let appBundle = URL(fileURLWithPath: CommandLine.arguments[0])
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let process = Process()
        process.executableURL = appBundle.appendingPathComponent("Contents/MacOS/pipboy")
        process.arguments = ["dashboard", "--port", "0"]
        let output = Pipe()
        process.standardOutput = output
        process.standardError = FileHandle.standardError
        do {
            try process.run()
            while process.isRunning {
                let data = output.fileHandleForReading.availableData
                guard !data.isEmpty, let text = String(data: data, encoding: .utf8) else { break }
                if let line = text.split(whereSeparator: \.isNewline)
                    .first(where: { $0.hasPrefix("Open http://127.0.0.1:") }),
                   let url = URL(string: String(line.dropFirst("Open ".count))),
                   let readyFile = ProcessInfo.processInfo.environment["RUSTBOY_READY_FILE"] {
                    let record = "\(url.absoluteString)\n\(process.processIdentifier)\n"
                    try? record.write(toFile: readyFile, atomically: true, encoding: .utf8)
                }
            }
            process.waitUntilExit()
        } catch {
            fputs("Rust-Boy headless smoke could not start: \(error.localizedDescription)\n", stderr)
        }
    }
}
