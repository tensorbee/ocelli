#!/usr/bin/env node
import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import process from 'node:process'
import console from 'node:console'
import { performance } from 'node:perf_hooks'
import { setTimeout as delay } from 'node:timers/promises'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const FORWARD_SIGNALS = ['SIGINT', 'SIGTERM']
const PLATFORMS = ['darwin', 'linux']

let current = null
let stopping = false
let signalExit = 0
let firstSignalHandled = false

const signalHandlers = new Map()

function makeHandler(exitCode) {
  return function () {
    if (firstSignalHandled) return
    firstSignalHandled = true
    stopping = true
    signalExit = exitCode
    if (current) current.stop(exitCode === 130 ? 'SIGINT' : 'SIGTERM')
  }
}

signalHandlers.set('SIGINT', makeHandler(130))
signalHandlers.set('SIGTERM', makeHandler(143))

for (const sig of FORWARD_SIGNALS) process.on(sig, signalHandlers.get(sig))

function removeSignalHandlers() {
  for (const sig of FORWARD_SIGNALS) process.removeListener(sig, signalHandlers.get(sig))
  signalHandlers.clear()
}

function fail(message) {
  console.error('folder-viewer-dev: ' + message)
}

async function stopOwnedGroup(ownedPid, signal, childClosed) {
  if (!Number.isInteger(ownedPid) || ownedPid <= 0 || ownedPid === process.pid) {
    throw new Error("invalid owned pid");
  }
  if (signal !== "SIGINT" && signal !== "SIGTERM") {
    throw new Error("invalid stop signal");
  }
  const probe = () => {
    try {
      process.kill(-ownedPid, 0);
      return true;
    } catch (err) {
      if (err.code === "ESRCH") return false;
      throw new Error("group probe failed");
    }
  };
  const sendToGroup = (sig) => {
    try {
      process.kill(-ownedPid, sig);
    } catch (err) {
      if (err.code !== "ESRCH") throw new Error("group signal failed");
    }
  };
  let childDone = false;
  let groupDone = false;
  if (!probe()) {
    groupDone = true;
  } else {
    sendToGroup(signal);
  }
  childClosed.then(() => {
    childDone = true;
  }, () => {});
  const start = performance.now();
  const grace = start + 2000;
  const hard = start + 4000;
  const waitForBoth = async (deadline) => {
    for (;;) {
      if (performance.now() >= deadline) return false;
      if (!groupDone) groupDone = !probe();
      if (childDone && groupDone) return true;
      await delay(20);
    }
  };
  if (await waitForBoth(grace)) {
    return;
  }
  if (!groupDone && probe()) sendToGroup("SIGKILL");
  if (await waitForBoth(hard)) {
    return;
  }
  throw new Error("owned group stop did not settle");
}

function startOwnedPhase(command, args, options) {
  let closedResolve;
  const closedPromise = new Promise((resolve) => { closedResolve = resolve; });
  let exitResolve;
  const exitPromise = new Promise((resolve) => { exitResolve = resolve; });
  let exitCode = 1;
  let spawnErr = false;
  let closed = false;
  let closeRecorded = false;
  let stopPromise = null;
  let firstSignal = null;
  const child = spawn(command, args, { ...options, detached: true });
  const hasValidPid =
    Number.isInteger(child.pid) && child.pid > 0 && child.pid !== process.pid;
  const recordClose = () => {
    if (closeRecorded) return;
    closeRecorded = true;
    closed = true;
    closedResolve();
  };
  const onClose = (code) => {
    if (!spawnErr) exitCode = typeof code === "number" ? code : exitCode;
    recordClose();
  };
  const onError = () => {
    spawnErr = true;
    exitCode = 1;
  };
  const onExit = () => {
    exitResolve();
  };
  child.on("close", onClose);
  child.on("error", onError);
  child.on("exit", onExit);
  function stop(signal) {
    if (stopPromise) return stopPromise;
    firstSignal = signal;
    if (hasValidPid) {
      stopPromise = (async () => {
        await stopOwnedGroup(child.pid, signal, closedPromise);
      })();
    } else {
      stopPromise = closedPromise;
    }
    stopPromise.catch(() => {});
    return stopPromise;
  }
  let signalStopCalled;
  const stopCalled = new Promise((resolve) => { signalStopCalled = resolve; });
  const wrappedStop = (signal) => {
    const p = stop(signal);
    signalStopCalled();
    return p;
  };
  const done = (async () => {
    await Promise.race([stopCalled, closedPromise, exitPromise]);
    if (!stopPromise) {
      if (hasValidPid) {
        await stop(firstSignal === null ? "SIGTERM" : firstSignal);
        signalStopCalled();
      } else {
        stopPromise = closedPromise;
        signalStopCalled();
      }
    }
    try {
      await stopPromise;
    } catch {
      child.removeListener("close", onClose);
      child.removeListener("error", onError);
      child.removeListener("exit", onExit);
      throw new Error("owned phase stop failed");
    }
    if (!closed) await closedPromise;
    child.removeListener("close", onClose);
    child.removeListener("error", onError);
    child.removeListener("exit", onExit);
    return exitCode;
  })();
  return { child, done, stop: wrappedStop };
}

async function runPhase(command, args, options) {
  const phase = startOwnedPhase(command, args, options)
  current = phase
  let code
  try {
    code = await phase.done
  } finally {
    if (current === phase) current = null
  }
  return code
}

async function runCoreBuild() {
  return await runPhase('npm', ['run', 'build', '-w', '@ocelli/core'], {
    cwd: ROOT,
    stdio: 'inherit'
  })
}

function parseCargoArtifacts(stdout) {
  const found = []
  for (const line of stdout.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed) continue
    let message
    try {
      message = JSON.parse(trimmed)
    } catch {
      continue
    }
    if (!message || message.reason !== 'compiler-artifact') continue
    const target = message.target
    if (!target || typeof target.name !== 'string') continue
    if (!Array.isArray(target.kind)) continue
    if (target.name !== 'ocelli-folder-viewer') continue
    if (!target.kind.includes('bin')) continue
    if (typeof message.executable !== 'string' || message.executable.length === 0) {
      fail('cargo reported a malformed binary artifact')
      return null
    }
    found.push(message.executable)
  }
  if (found.length === 0) {
    fail('cargo produced no ocelli-folder-viewer binary artifact')
    return null
  }
  if (found.length > 1) {
    fail('cargo produced more than one matching binary artifact')
    return null
  }
  return found[0]
}

async function runCargoBuild() {
  const phase = startOwnedPhase(
    'cargo',
    ['build', '-p', 'ocelli-folder-viewer', '--message-format=json'],
    { cwd: ROOT, stdio: ['ignore', 'pipe', 'inherit'] }
  )
  current = phase
  let stdout = ''
  phase.child.stdout.setEncoding('utf8')
  phase.child.stdout.on('data', (chunk) => {
    stdout += chunk
  })
  let code
  try {
    code = await phase.done
  } finally {
    if (current === phase) current = null
  }
  if (code !== 0) return { code, executable: null }
  return { code: 0, executable: parseCargoArtifacts(stdout) }
}

async function main() {
  if (!PLATFORMS.includes(process.platform)) {
    fail('folder-viewer-dev only supports macOS and Linux')
    process.exitCode = 1
    return
  }

  const npmCode = await runCoreBuild()
  if (stopping) return
  if (npmCode !== 0) {
    fail('core build failed')
    process.exitCode = 1
    return
  }

  const cargo = await runCargoBuild()
  if (stopping) return
  if (cargo.code !== 0) {
    fail('folder-viewer build failed')
    process.exitCode = 1
    return
  }
  if (cargo.executable === null) {
    process.exitCode = 1
    return
  }

  const backendArgs = process.argv.slice(2)
  const code = await runPhase(cargo.executable, backendArgs, {
    cwd: ROOT,
    stdio: 'inherit'
  })
  if (stopping) return
  process.exitCode = code === 0 ? 0 : 1
}

try {
  await main()
} catch {
  fail('folder-viewer-dev encountered an internal error')
  if (!stopping) process.exitCode = 1
} finally {
  removeSignalHandlers()
  if (stopping) process.exitCode = signalExit
}
