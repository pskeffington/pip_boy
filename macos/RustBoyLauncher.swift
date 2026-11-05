import AppKit
import Darwin
import Foundation

@MainActor
final class RustBoyAppDelegate: NSObject, NSApplicationDelegate {
    private var statusItem: NSStatusItem!
    private var server: Process?
    private var dashboardURL: URL?
    private var pendingPath = "/"
    private var outputBuffer = ""
    private var lifecycleSmokeScheduled = false
    private var liveQuitSmokeScheduled = false
    private var failureSmokeScheduled = false
    private var launchFailure: String?
    private var isStoppingServer = false
    private var stopCompletion: (() -> Void)?

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

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard server?.isRunning == true else { return .terminateNow }
        let childPID = server?.processIdentifier ?? 0
        stopServer {
            if ProcessInfo.processInfo.environment["RUSTBOY_QUIT_LIVE_SMOKE"] == "1",
               let resultFile = ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_RESULT_FILE"] {
                self.appendSmokeResult("quit_stopped_child_pid=\(childPID)", to: resultFile)
            }
            NSApp.terminate(nil)
        }
        return .terminateCancel
    }

    @objc private func openDashboard() { open(path: "/") }
    @objc private func openMicroHUD() { open(path: "/micro") }
    @objc private func stopDashboard() {
        let childPID = server?.processIdentifier ?? 0
        guard ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_SMOKE"] == "1",
              let resultFile = ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_RESULT_FILE"] else {
            stopServer()
            return
        }
        stopServer { [weak self] in
            guard let self else { return }
            self.appendSmokeResult("stopped_child_pid=\(childPID)", to: resultFile)
            NSApp.terminate(nil)
        }
    }
    @objc private func quit() {
        if ProcessInfo.processInfo.environment["RUSTBOY_QUIT_LIVE_SMOKE"] == "1",
           let resultFile = ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_RESULT_FILE"] {
            appendSmokeResult("quit_action=true", to: resultFile)
        }
        NSApp.terminate(nil)
    }

    private func open(path: String) {
        pendingPath = path
        guard let dashboardURL else {
            startServer()
            return
        }
        if ProcessInfo.processInfo.environment["RUSTBOY_DISABLE_BROWSER_OPEN"] == "1" {
            return
        }
        if let target = URL(string: path, relativeTo: dashboardURL)?.absoluteURL {
            NSWorkspace.shared.open(target)
        }
    }

    @objc private func startServer() {
        guard server == nil else {
            if dashboardURL == nil {
                pendingPath = "/"
            }
            return
        }
        let executable = Bundle.main.bundleURL
            .appendingPathComponent("Contents/MacOS/pipboy", isDirectory: false)
        launchFailure = nil
        outputBuffer = ""
        let process = Process()
        process.executableURL = executable
        process.arguments = ["dashboard", "--port", "0"]
        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors
        process.terminationHandler = { [weak self] _ in
            DispatchQueue.main.async {
                self?.processDidTerminate(process)
            }
        }
        do {
            try process.run()
            server = process
            let outputHandle = output.fileHandleForReading
            DispatchQueue.global(qos: .utility).async { [weak self, outputHandle] in
                while true {
                    let data = outputHandle.availableData
                    guard !data.isEmpty, let text = String(data: data, encoding: .utf8) else { break }
                    DispatchQueue.main.async { self?.consumeOutput(text) }
                }
            }
            let errorHandle = errors.fileHandleForReading
            DispatchQueue.global(qos: .utility).async { [errorHandle] in
                while !errorHandle.availableData.isEmpty {}
            }
            rebuildMenu()
        } catch {
            server = nil
            launchFailure = "Dashboard could not start: \(error.localizedDescription)"
            NSLog("Rust-Boy dashboard failed to start: %@", error.localizedDescription)
            rebuildMenu()
            scheduleFailureSmokeIfNeeded()
        }
    }

    private func consumeOutput(_ text: String) {
        outputBuffer += text
        let lines = outputBuffer.components(separatedBy: "\n")
        outputBuffer = lines.last ?? ""
        guard let line = lines.dropLast().first(where: { $0.hasPrefix("Open ") }),
              let url = DashboardEndpoint.rootURL(from: line) else {
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

    private func stopServer(completion: (() -> Void)? = nil) {
        guard let process = server else {
            completion?()
            return
        }
        if isStoppingServer {
            if let completion {
                let previousCompletion = stopCompletion
                stopCompletion = {
                    previousCompletion?()
                    completion()
                }
            }
            return
        }
        stopCompletion = completion
        guard process.isRunning else {
            processDidTerminate(process)
            return
        }
        isStoppingServer = true
        dashboardURL = nil
        launchFailure = nil
        rebuildMenu()
        process.terminate()
        DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + 3) {
            guard process.isRunning else { return }
            NSLog("Rust-Boy dashboard did not stop after SIGTERM; sending SIGKILL.")
            _ = Darwin.kill(process.processIdentifier, SIGKILL)
        }
    }

    private func processDidTerminate(_ process: Process) {
        guard server === process else { return }
        let requestedStop = isStoppingServer
        server = nil
        dashboardURL = nil
        isStoppingServer = false
        if !requestedStop, process.terminationStatus != 0,
           ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_SMOKE"] != "1" {
            launchFailure = "Dashboard stopped unexpectedly (\(process.terminationStatus))."
        }
        let completion = stopCompletion
        stopCompletion = nil
        rebuildMenu()
        completion?()
    }

    private func rebuildMenu() {
        let menu = NSMenu()
        let title: String
        if let launchFailure {
            title = launchFailure.hasPrefix("Dashboard could not start")
                ? "Dashboard unavailable — retry below"
                : launchFailure
        } else if dashboardURL != nil {
            title = "Dashboard running on this Mac"
        } else if isStoppingServer {
            title = "Stopping local dashboard…"
        } else if server != nil {
            title = "Starting local dashboard…"
        } else {
            title = "Dashboard stopped"
        }
        let state = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        state.isEnabled = false
        menu.addItem(state)
        menu.addItem(.separator())
        if server == nil {
            menu.addItem(NSMenuItem(title: "Retry Dashboard", action: #selector(startServer), keyEquivalent: ""))
        }
        menu.addItem(NSMenuItem(title: "Open Desktop Dashboard", action: #selector(openDashboard), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Open Field HUD", action: #selector(openMicroHUD), keyEquivalent: ""))
        let stopItem = NSMenuItem(title: "Stop Dashboard", action: #selector(stopDashboard), keyEquivalent: "")
        stopItem.isEnabled = server != nil && !isStoppingServer
        menu.addItem(stopItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit Rust-Boy", action: #selector(quit), keyEquivalent: "q"))
        for item in menu.items where item.action != nil { item.target = self }
        statusItem.menu = menu
        if !failureSmokeScheduled, launchFailure != nil,
           ProcessInfo.processInfo.environment["RUSTBOY_FAILURE_SMOKE"] == "1" {
            scheduleFailureSmokeIfNeeded()
        }
        if !liveQuitSmokeScheduled,
           dashboardURL != nil,
           ProcessInfo.processInfo.environment["RUSTBOY_QUIT_LIVE_SMOKE"] == "1" {
            liveQuitSmokeScheduled = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
                guard let self else { return }
                if !self.dispatchMenuAction(named: "Quit Rust-Boy") {
                    self.appendSmokeResult(
                        "quit_action=false",
                        to: ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_RESULT_FILE"] ?? ""
                    )
                }
            }
        }
        guard !lifecycleSmokeScheduled,
              dashboardURL != nil,
              ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_SMOKE"] == "1" else {
            return
        }
        lifecycleSmokeScheduled = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
            guard let self else { return }
            let desktopOpened = self.dispatchMenuAction(named: "Open Desktop Dashboard")
            let microOpened = self.dispatchMenuAction(named: "Open Field HUD")
            let stopDispatched = self.dispatchMenuAction(named: "Stop Dashboard")
            self.appendSmokeResult(
                "desktop_action=\(desktopOpened)\nmicro_action=\(microOpened)\nstop_action=\(stopDispatched)",
                to: ProcessInfo.processInfo.environment["RUSTBOY_LIFECYCLE_RESULT_FILE"] ?? ""
            )
        }
    }

    private func dispatchMenuAction(named title: String) -> Bool {
        guard let item = statusItem.menu?.item(withTitle: title),
              let action = item.action else {
            return false
        }
        return NSApp.sendAction(action, to: item.target, from: item)
    }

    private func appendSmokeResult(_ line: String, to path: String) {
        guard !path.isEmpty else { return }
        let url = URL(fileURLWithPath: path)
        let data = Data((line + "\n").utf8)
        if FileManager.default.fileExists(atPath: path),
           let handle = try? FileHandle(forWritingTo: url) {
            defer { try? handle.close() }
            _ = try? handle.seekToEnd()
            try? handle.write(contentsOf: data)
        } else {
            try? data.write(to: url, options: .atomic)
        }
    }

    private func scheduleFailureSmokeIfNeeded() {
        guard !failureSmokeScheduled,
              ProcessInfo.processInfo.environment["RUSTBOY_FAILURE_SMOKE"] == "1" else {
            return
        }
        failureSmokeScheduled = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
            guard let self else { return }
            let retryDispatched = self.dispatchMenuAction(named: "Retry Dashboard")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                let stateTitle = self.statusItem.menu?.items.first?.title ?? ""
                let retryAvailable = self.statusItem.menu?.item(withTitle: "Retry Dashboard")?.isEnabled == true
                let result = "retry_dispatched=\(retryDispatched)\nretry_available=\(retryAvailable)\nstate=\(stateTitle)"
                self.appendSmokeResult(result, to: ProcessInfo.processInfo.environment["RUSTBOY_FAILURE_RESULT_FILE"] ?? "")
                _ = self.dispatchMenuAction(named: "Quit Rust-Boy")
            }
        }
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
        let appBundle: URL
        if let configuredPath = ProcessInfo.processInfo.environment["RUSTBOY_APP_BUNDLE_PATH"] {
            appBundle = URL(fileURLWithPath: configuredPath)
        } else {
            appBundle = URL(fileURLWithPath: CommandLine.arguments[0])
                .deletingLastPathComponent()
                .deletingLastPathComponent()
                .deletingLastPathComponent()
        }
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
                    .first(where: { $0.hasPrefix("Open ") }),
                   let url = DashboardEndpoint.rootURL(from: String(line)),
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
