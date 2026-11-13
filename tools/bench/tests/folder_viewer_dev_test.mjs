import { test } from 'node:test'
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'
import process from 'node:process'

const repoBin = path.resolve(import.meta.dirname, '../../../bin/folder-viewer-dev.mjs')
const SIGNALS = ['SIGINT', 'SIGTERM']
const READY_MS = 5000
const CLEANUP_MS = 5000

function alive(pid) {
  try { process.kill(pid, 0); return true } catch (e) {
    if (e.code === 'ESRCH') return false
    throw e
  }
}

async function waitForFile(file, what) {
  const end = Date.now() + READY_MS
  while (Date.now() < end) {
    if (fs.existsSync(file)) return
    await delay(20)
  }
  throw new Error('timeout waiting for ' + what)
}

async function syntaxCheck(file) {
  const check = spawn(process.execPath, ['--check', file], { stdio: 'inherit' })
  await new Promise((resolve, reject) => {
    check.on('error', reject)
    check.on('close', (code) => code === 0 ? resolve() : reject(new Error('fixture failed --check: ' + file)))
  })
}

async function buildFixtures(temp) {
  const fakeBin = path.join(temp, 'fakebin')
  fs.mkdirSync(fakeBin)
  const target = path.join(temp, 'custom target dir')
  fs.mkdirSync(target)
  fs.mkdirSync(path.join(temp, 'repo', 'bin'), { recursive: true })
  const launcher = path.join(temp, 'repo', 'bin', 'folder-viewer-dev.mjs')
  fs.copyFileSync(repoBin, launcher)
  assert.equal(fs.readFileSync(repoBin, 'utf8'), fs.readFileSync(launcher, 'utf8'), 'launcher copy is byte-identical')

  const viewer = path.join(target, 'fake-viewer.mjs')
  fs.writeFileSync(viewer, `#!/usr/bin/env node
import fs from 'node:fs'
const log = process.env.FAKE_SIGNAL_LOG
fs.writeFileSync(process.env.FAKE_MARKER, JSON.stringify({ pid: process.pid, script: import.meta.filename }))
const timer = setInterval(() => {}, 50)
for (const sig of ['SIGINT', 'SIGTERM']) {
  process.on(sig, () => {
    clearInterval(timer)
    fs.writeFileSync(log, sig)
    process.exit(0)
  })
}
`)
  fs.chmodSync(viewer, 0o755)

  const descendantScript = path.join(fakeBin, 'descendant-script.mjs')
  fs.writeFileSync(descendantScript, `#!/usr/bin/env node
import fs from 'node:fs'
import process from 'node:process'
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => {})
fs.writeFileSync(process.env.DESCENDANT_MARKER, JSON.stringify(process.pid))
setInterval(() => {}, 1000)
`)
  fs.chmodSync(descendantScript, 0o755)
  const npm = path.join(fakeBin, 'npm')
  fs.writeFileSync(npm, `#!/usr/bin/env node
import fs from 'node:fs'
import { spawn } from 'node:child_process'
import process from 'node:process'
fs.writeFileSync(process.env.FAKE_NPM_LOG, JSON.stringify(process.argv.slice(2)))
const mode = process.env.FAKE_NPM_MODE
if (mode === 'hold') {
  for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => {})
  const child = spawn(process.execPath, [process.env.FAKE_DESCENDANT_SCRIPT], { stdio: 'ignore' })
  child.unref()
  fs.writeFileSync(process.env.PARENT_MARKER, JSON.stringify(process.pid))
  setInterval(() => {}, 1000)
} else if (mode === 'orphan') {
  for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => {})
  const child = spawn(process.execPath, [process.env.FAKE_DESCENDANT_SCRIPT], { stdio: 'ignore' })
  child.unref()
  fs.writeFileSync(process.env.PARENT_MARKER, JSON.stringify(process.pid))
  const start = Date.now()
  let ready = false
  while (Date.now() - start < 5000) {
    if (fs.existsSync(process.env.DESCENDANT_MARKER)) { ready = true; break }
    await new Promise((resolve) => setTimeout(resolve, 50))
  }
  if (!ready) process.exit(1)
} else {
  process.exit(0)
}
`)
  fs.chmodSync(npm, 0o755)

  const cargo = path.join(fakeBin, 'cargo')
  fs.writeFileSync(cargo, `#!/usr/bin/env node
import fs from 'node:fs'
fs.writeFileSync(process.env.FAKE_CARGO_LOG, JSON.stringify({ argv: process.argv.slice(2), dir: process.env.CARGO_TARGET_DIR }))
const dep = { reason: 'compiler-artifact', target: { name: 'some-dep', kind: ['lib'] } }
const binArtifact = { reason: 'compiler-artifact', target: { name: 'ocelli-folder-viewer', kind: ['bin'] }, executable: process.env.FAKE_VIEWER }
console.log(JSON.stringify(dep))
console.log(JSON.stringify(binArtifact))
`)
  fs.chmodSync(cargo, 0o755)

  fs.writeFileSync(path.join(temp, 'repo', 'package.json'), JSON.stringify({ type: 'module' }))

  for (const fixture of [viewer, npm, descendantScript, cargo, launcher]) await syntaxCheck(fixture)

  return {
    fakeBin, target, launcher, viewer,
    npmLog: path.join(temp, 'npm-args.json'),
    cargoLog: path.join(temp, 'cargo-args.json'),
    marker: path.join(temp, 'viewer-marker.json'),
    signalLog: path.join(temp, 'viewer-signals.log'),
    npmMarker: path.join(temp, 'npm-marker.json'),
    descendantMarker: path.join(temp, 'descendant-marker.json'),
    descendantScript
  }
}

function spawnLauncher(temp, fx, extraEnv = {}) {
  const child = spawn(process.execPath, [fx.launcher], {
    cwd: temp,
    detached: true,
    env: {
      ...process.env,
      PATH: fx.fakeBin + path.delimiter + path.dirname(process.execPath) + path.delimiter + process.env.PATH,
      CARGO_TARGET_DIR: fx.target,
      FAKE_NPM_LOG: fx.npmLog,
      FAKE_CARGO_LOG: fx.cargoLog,
      FAKE_VIEWER: fx.viewer,
      FAKE_MARKER: fx.marker,
      FAKE_SIGNAL_LOG: fx.signalLog,
      PARENT_MARKER: fx.npmMarker,
      DESCENDANT_MARKER: fx.descendantMarker,
      FAKE_DESCENDANT_SCRIPT: fx.descendantScript,
      ...extraEnv
    },
    stdio: 'ignore'
  })
  const closed = new Promise((resolve) => child.once('close', resolve))
  const errored = new Promise((resolve) => child.once('error', resolve))
  return { child, closed, errored }
}

async function stopOwned(run, launcherPid, viewerPid) {
  const errors = []
  try {
    if (launcherPid) process.kill(-launcherPid, 'SIGKILL')
  } catch (e) {
    if (e.code !== 'ESRCH') errors.push(e)
  }
  try {
    if (viewerPid && viewerPid !== launcherPid) process.kill(viewerPid, 'SIGKILL')
  } catch (e) {
    if (e.code !== 'ESRCH') errors.push(e)
  }
  if (run) {
    let timer
    const timeout = new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error('owned launcher did not close within ' + CLEANUP_MS + 'ms')), CLEANUP_MS)
    })
    try {
      await Promise.race([run.closed, run.errored, timeout])
    } finally {
      clearTimeout(timer)
    }
  }
  if (errors.length) throw errors[0]
}

for (const sig of SIGNALS) {
  test('launcher forwards ' + sig + ' to cargo-selected viewer and cleans up own children', async () => {
    if (!fs.existsSync(repoBin)) {
      assert.fail('MISSING_IMPLEMENTATION: bin/folder-viewer-dev.mjs does not exist yet')
    }
    const temp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'f245-launcher-')))
    let run = null
    let launcherPid = 0
    let viewerPid = 0
    let primaryError = null
    try {
      const fx = await buildFixtures(temp)
      run = spawnLauncher(temp, fx)
      launcherPid = run.child.pid
      await waitForFile(fx.marker, 'viewer start marker')
      const marker = JSON.parse(fs.readFileSync(fx.marker, 'utf8'))
      assert.equal(marker.script, fx.viewer, 'marker proves the viewer ran from the cargo-selected fixture path')
      assert.ok(Number.isInteger(marker.pid) && marker.pid > 0, 'viewer reported a valid pid')
      viewerPid = marker.pid
      assert.notEqual(viewerPid, launcherPid, 'captured pid belongs to the fake viewer child')
      assert.ok(alive(viewerPid), 'fake viewer is live before signal')
      assert.ok(alive(launcherPid), 'launcher still running before signal')

      const npmArgs = JSON.parse(fs.readFileSync(fx.npmLog, 'utf8'))
      assert.deepEqual(npmArgs, ['run', 'build', '-w', '@ocelli/core'])
      const cargo = JSON.parse(fs.readFileSync(fx.cargoLog, 'utf8'))
      assert.deepEqual([...cargo.argv].sort(), ['--message-format=json', '-p', 'ocelli-folder-viewer', 'build'].sort())
      assert.equal(cargo.dir, fx.target)

      process.kill(launcherPid, sig)
      await waitForFile(fx.signalLog, 'viewer received signal')
      assert.equal(fs.readFileSync(fx.signalLog, 'utf8'), sig)
      const closeCode = await awaitClose(run.closed, 6000, 'launcher close after ' + sig)
      assert.equal(closeCode, sig === 'SIGINT' ? 130 : 143)
      await delay(50)
      assert.ok(!alive(viewerPid), 'no owned viewer child left')
      assert.ok(!alive(launcherPid), 'launcher closed after forwarding signal')
    } catch (e) {
      primaryError = e
    }
    try {
      await stopOwned(run, launcherPid, viewerPid)
    } catch (cleanupErr) {
      if (!primaryError) primaryError = cleanupErr
    }
    fs.rmSync(temp, { recursive: true, force: true })
    if (primaryError) throw primaryError
  })
}

// Regression: stop during npm build while the fake npm parent holds its
// same-group grandchild. First SIGINT then SIGTERM to the launcher must exit
// 130, the npm phase group (parent plus grandchild) must be gone, and a
// separately owned detached sentinel must survive. Cleanup signals only
// exact owned groups and never the grandchild PID.

function readMarkerPid(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'))
}

function assertOwnedPid(pid, label) {
  assert.ok(Number.isInteger(pid) && pid > 0, label + ' pid must be a positive integer')
  assert.notStrictEqual(pid, process.pid, label + ' pid must not be the test process')
}

function isGroupLeader(pid) {
  try { process.kill(-pid, 0); return true } catch (e) {
    if (e.code === 'ESRCH') return false
    throw e
  }
}

async function awaitClose(closePromise, ms, label) {
  let timer
  const timeout = new Promise((resolve, reject) => {
    timer = setTimeout(() => reject(new Error(label + ' close timeout')), ms)
  })
  try { return await Promise.race([closePromise, timeout]) } finally { clearTimeout(timer) }
}

async function killOwnedGroup(pid, label) {
  assertOwnedPid(pid, label)
  try { process.kill(-pid, 'SIGKILL') } catch (e) {
    if (e.code !== 'ESRCH') throw e
  }
}

async function pollGone(pid, label, isGroupLeaderRequired) {
  const end = Date.now() + 5000
  while (Date.now() < end) {
    if (!alive(pid) && (!isGroupLeaderRequired || !isGroupLeader(pid))) return
    await delay(50)
  }
  throw new Error(label + ' still alive after bounded cleanup deadline')
}

test('stop during npm build with held same-group grandchild exits 130 on INT then TERM and owned sentinel survives', async () => {
  const temp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'fv-stop-hold-')))
  const fx = await buildFixtures(temp)
  const sentinel = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { detached: true, stdio: 'ignore' })
  const sentinelClosed = new Promise((resolve) => sentinel.once('close', resolve))
  const sentinelPid = sentinel.pid

  let primaryError = null
  let cleanupError = null
  let launcher = null
  try {
    assertOwnedPid(sentinelPid, 'sentinel')
    assert.ok(alive(sentinelPid), 'sentinel alive after spawn')

    launcher = spawnLauncher(temp, fx, { FAKE_NPM_MODE: 'hold' })
    const launcherPid = launcher.child.pid
    assertOwnedPid(launcherPid, 'launcher')

    await waitForFile(fx.npmMarker, 'npm marker')
    await waitForFile(fx.descendantMarker, 'descendant marker')
    const npmPid = readMarkerPid(fx.npmMarker)
    const descendantPid = readMarkerPid(fx.descendantMarker)
    assertOwnedPid(npmPid, 'npm')
    assertOwnedPid(descendantPid, 'descendant')
    assert.ok(alive(launcherPid), 'launcher alive')
    assert.ok(alive(npmPid), 'npm alive')
    assert.ok(alive(descendantPid), 'descendant alive')
    assert.ok(alive(sentinelPid), 'sentinel alive')
    const distinct = new Set([launcherPid, npmPid, descendantPid, sentinelPid])
    assert.strictEqual(distinct.size, 4, 'launcher, npm, descendant, sentinel pids distinct')
    assert.ok(isGroupLeader(npmPid), 'npm owns its phase group')
    assert.ok(!isGroupLeader(descendantPid), 'descendant does not own a group')

    process.kill(launcherPid, 'SIGINT')
    await delay(100)
    assert.ok(alive(npmPid), 'npm ignores INT while holding')
    assert.ok(alive(descendantPid), 'grandchild ignores INT while held')

    process.kill(launcherPid, 'SIGTERM')
    const code = await awaitClose(launcher.closed, 6000, 'launcher after TERM')
    assert.strictEqual(code, 130, 'launcher exits 130 after INT then TERM')

    assert.ok(!fs.existsSync(fx.cargoLog), 'no cargo log written')
    assert.ok(!fs.existsSync(fx.marker), 'no viewer marker written')
    assert.ok(!isGroupLeader(npmPid) && !alive(npmPid), 'npm phase group and npm pid gone')
    assert.ok(!alive(descendantPid), 'held grandchild gone with its group')
    assert.ok(alive(sentinelPid), 'owned sentinel still alive after stop')
  } catch (e) {
    primaryError = e
  }

  try {
    let npmPid = null
    let descendantPid = null
    if (fs.existsSync(fx.npmMarker)) npmPid = readMarkerPid(fx.npmMarker)
    if (fs.existsSync(fx.descendantMarker)) descendantPid = readMarkerPid(fx.descendantMarker)
    if (launcher) await killOwnedGroup(launcher.child.pid, 'launcher group')
    if (npmPid) await killOwnedGroup(npmPid, 'npm group')
    await killOwnedGroup(sentinelPid, 'sentinel group')
    await awaitClose(launcher ? launcher.closed : Promise.resolve(), 3000, 'launcher cleanup close')
    await awaitClose(sentinelClosed, 3000, 'sentinel cleanup close')
    if (launcher) await pollGone(launcher.child.pid, 'launcher')
    if (npmPid) await pollGone(npmPid, 'npm group', true)
    await pollGone(sentinelPid, 'sentinel group', true)
    if (npmPid && descendantPid && npmPid === descendantPid) {
      throw new Error('fixture markers collapsed to one pid, refusing cleanup')
    }
    fs.rmSync(temp, { recursive: true, force: true })
  } catch (e) {
    cleanupError = cleanupError || e
  }

  if (primaryError) throw primaryError
  if (cleanupError) throw cleanupError
})

test('fake viewer exits 0 and launcher exits 143 while viewer receives TERM, with normal npm child group close cleanup before cargo and the unrelated owned sentinel surviving', async () => {
  const temp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'fv-stop-orphan-')))
  const fx = await buildFixtures(temp)
  const sentinel = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { detached: true, stdio: 'ignore' })
  const sentinelClosed = new Promise((resolve) => sentinel.once('close', resolve))
  const sentinelPid = sentinel.pid

  let primaryError = null
  let cleanupError = null
  let launcher = null
  try {
    assertOwnedPid(sentinelPid, 'sentinel')
    assert.ok(alive(sentinelPid), 'sentinel alive after spawn')

    launcher = spawnLauncher(temp, fx, { FAKE_NPM_MODE: 'orphan' })
    const launcherPid = launcher.child.pid
    assertOwnedPid(launcherPid, 'launcher')

    await waitForFile(fx.npmMarker, 'npm marker')
    await waitForFile(fx.descendantMarker, 'descendant marker')
    const npmPid = readMarkerPid(fx.npmMarker)
    const descendantPid = readMarkerPid(fx.descendantMarker)
    assertOwnedPid(npmPid, 'npm')
    assertOwnedPid(descendantPid, 'descendant')
    assert.ok(alive(launcherPid), 'launcher alive')
    assert.ok(alive(descendantPid), 'descendant alive')
    assert.ok(alive(sentinelPid), 'sentinel alive')
    const distinct = new Set([launcherPid, npmPid, descendantPid, sentinelPid])
    assert.strictEqual(distinct.size, 4, 'launcher, npm, descendant, sentinel pids distinct')
    assert.ok(isGroupLeader(npmPid), 'npm owns its phase group')
    assert.ok(!isGroupLeader(descendantPid), 'descendant does not own a group')

    const deadline = Date.now() + 5000
    while (alive(npmPid) && Date.now() < deadline) await delay(20)
    assert.ok(!alive(npmPid), 'npm closed directly in orphan mode')
    assert.ok(alive(descendantPid), 'orphaned descendant survives npm close')
    assert.ok(!fs.existsSync(fx.cargoLog), 'no cargo log before viewer writes')

    await waitForFile(fx.cargoLog, 'cargo log')
    await waitForFile(fx.marker, 'viewer marker')
    const viewerPid = JSON.parse(fs.readFileSync(fx.marker, 'utf8')).pid
    assertOwnedPid(viewerPid, 'viewer')
    assert.ok(!isGroupLeader(npmPid), 'npm phase group gone after direct close')
    assert.ok(!alive(descendantPid), 'descendant gone after cargo log written')
    assert.ok(alive(sentinelPid), 'owned sentinel still alive')

    process.kill(launcherPid, 'SIGTERM')
    const code = await awaitClose(launcher.closed, 6000, 'launcher after TERM')
    assert.strictEqual(code, 143, 'launcher exits 143 after TERM')
    assert.ok(!isGroupLeader(viewerPid) && !alive(viewerPid), 'viewer pid and group gone')
  } catch (e) {
    primaryError = e
  }

  try {
    let npmPid = null
    let descendantPid = null
    let viewerPid = null
    if (fs.existsSync(fx.npmMarker)) npmPid = readMarkerPid(fx.npmMarker)
    if (fs.existsSync(fx.descendantMarker)) descendantPid = readMarkerPid(fx.descendantMarker)
    if (fs.existsSync(fx.marker)) viewerPid = JSON.parse(fs.readFileSync(fx.marker, 'utf8')).pid
    if (launcher) await killOwnedGroup(launcher.child.pid, 'launcher group')
    if (npmPid) await killOwnedGroup(npmPid, 'npm group')
    if (viewerPid) await killOwnedGroup(viewerPid, 'viewer group')
    await killOwnedGroup(sentinelPid, 'sentinel group')
    await awaitClose(launcher ? launcher.closed : Promise.resolve(), 3000, 'launcher cleanup close')
    await awaitClose(sentinelClosed, 3000, 'sentinel cleanup close')
    if (launcher) await pollGone(launcher.child.pid, 'launcher')
    if (npmPid) await pollGone(npmPid, 'npm group')
    if (viewerPid) await pollGone(viewerPid, 'viewer group')
    await pollGone(sentinelPid, 'sentinel group')
    if (npmPid && descendantPid && npmPid === descendantPid) {
      throw new Error('fixture markers collapsed to one pid, refusing cleanup')
    }
    fs.rmSync(temp, { recursive: true, force: true })
  } catch (e) {
    cleanupError = cleanupError || e
  }

  if (primaryError) throw primaryError
  if (cleanupError) throw cleanupError
})

test('cargo exit with stdout-inheriting descendant triggers owned cleanup within bound and backend advances', async () => {
  const temp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'fv-cargo-stdout-desc-')))
  const fx = await buildFixtures(temp)
  const cargoPath = path.join(fx.fakeBin, 'cargo')
  fs.writeFileSync(cargoPath, [
    '#!/usr/bin/env node',
    'import fs from "node:fs"',
    'import { spawn } from "node:child_process"',
    'fs.writeFileSync(process.env.PARENT_MARKER, JSON.stringify(process.pid))',
    'const child = spawn(process.execPath, [process.env.FAKE_DESCENDANT_SCRIPT], { stdio: ["ignore", "inherit", "ignore"] })',
    'child.unref()',
    'const deadline = Date.now() + 5000',
    'while (!fs.existsSync(process.env.DESCENDANT_MARKER) && Date.now() < deadline) await new Promise((resolve) => setTimeout(resolve, 20))',
    'if (!fs.existsSync(process.env.DESCENDANT_MARKER)) { process.exit(1) }',
    'const artifact = {',
    '  reason: "compiler-artifact",',
    '  target: { name: "ocelli-folder-viewer", kind: ["bin"] },',
    '  executable: process.env.FAKE_VIEWER',
    '}',
    'console.log(JSON.stringify(artifact))'
  ].join('\n'))
  await syntaxCheck(cargoPath)

  let primaryError = null
  let cleanupError = null
  let launcher = null
  let cargoPid = null
  let descendantPid = null
  let viewerPid = null
  try {
    launcher = spawnLauncher(temp, fx)
    const launcherPid = launcher.child.pid
    assertOwnedPid(launcherPid, 'launcher')

    await waitForFile(fx.npmMarker, 'cargo parent marker')
    await waitForFile(fx.descendantMarker, 'descendant marker')
    cargoPid = readMarkerPid(fx.npmMarker)
    descendantPid = readMarkerPid(fx.descendantMarker)
    assertOwnedPid(cargoPid, 'cargo')
    assertOwnedPid(descendantPid, 'descendant')
    assert.ok(cargoPid !== descendantPid, 'cargo and descendant pids distinct')
    assert.ok(cargoPid !== launcherPid && descendantPid !== launcherPid, 'fixture pids distinct from launcher')
    assert.ok(isGroupLeader(cargoPid), 'cargo owns its phase group')
    assert.ok(!isGroupLeader(descendantPid), 'descendant does not own a group')

    const cargoExitDeadline = Date.now() + 5000
    while (alive(cargoPid) && Date.now() < cargoExitDeadline) await delay(20)
    assert.ok(!alive(cargoPid), 'cargo pid exited while descendant retains piped stdout')

    await delay(4500)
    assert.ok(!alive(descendantPid), 'Cargo stdout-holding descendant must be gone within the owned cleanup bound')
    assert.ok(!isGroupLeader(cargoPid), 'cargo phase group gone after owned cleanup')

    await waitForFile(fx.marker, 'viewer marker')
    viewerPid = JSON.parse(fs.readFileSync(fx.marker, 'utf8')).pid
    assertOwnedPid(viewerPid, 'viewer')

    process.kill(launcherPid, 'SIGTERM')
    const code = await awaitClose(launcher.closed, 6000, 'launcher after TERM')
    assert.strictEqual(code, 143, 'launcher exits 143 after TERM')
  } catch (e) {
    primaryError = e
  }

  try {
    if (fs.existsSync(fx.npmMarker) && cargoPid === null) cargoPid = readMarkerPid(fx.npmMarker)
    if (fs.existsSync(fx.descendantMarker) && descendantPid === null) descendantPid = readMarkerPid(fx.descendantMarker)
    if (fs.existsSync(fx.marker) && viewerPid === null) viewerPid = JSON.parse(fs.readFileSync(fx.marker, 'utf8')).pid
    if (launcher) await killOwnedGroup(launcher.child.pid, 'launcher group')
    if (cargoPid) await killOwnedGroup(cargoPid, 'cargo group')
    if (viewerPid) await killOwnedGroup(viewerPid, 'viewer group')
    await awaitClose(launcher ? launcher.closed : Promise.resolve(), 3000, 'launcher cleanup close')
    if (launcher) await pollGone(launcher.child.pid, 'launcher')
    if (cargoPid) await pollGone(cargoPid, 'cargo group')
    if (viewerPid) await pollGone(viewerPid, 'viewer group')
    if (cargoPid && descendantPid && cargoPid === descendantPid) {
      throw new Error('fixture markers collapsed to one pid, refusing cleanup')
    }
    fs.rmSync(temp, { recursive: true, force: true })
  } catch (e) {
    cleanupError = cleanupError || e
  }

  if (primaryError) throw primaryError
  if (cleanupError) throw cleanupError
})
