# Ainux Desktop Sovereign OS Transformation Directive

The Ainux project is now transitioning from an experimental runtime-oriented kernel prototype into a complete independent desktop operating system project.

The goal is NO LONGER:

* a minimal research kernel,
* a runtime experiment,
* or a lightweight subsystem demo.

The new objective is:

“Transform Ainux into a complete sovereign desktop operating system with its own runtime architecture, desktop environment, service ecosystem, execution model, and application platform.”

This operating system should evolve similarly to:

* SkyOS,
* Inferno,
* Plan 9,
* BeOS,
* and capability-secure runtime systems,

while remaining architecturally unique.

# Strategic Transformation

We are NOT:

* cloning Linux,
* copying Windows,
* or creating another Unix derivative.

We ARE building:

* a fully independent operating environment,
* with its own execution model,
* service architecture,
* runtime philosophy,
* desktop environment,
* and capability-oriented infrastructure.

# Core Philosophy

Ainux is now defined as:

“A sovereign desktop operating environment where the runtime graph defines the operating system.”

The system must combine:

* desktop usability,
* runtime isolation,
* namespace mediation,
* service modularity,
* lightweight execution,
* and recoverable system topology.

# Architectural Identity

Ainux should become:

## 1. A Complete Desktop Operating System

Including:

* desktop environment,
* window manager,
* shell,
* launcher,
* task manager,
* settings,
* file explorer,
* software runtime ecosystem,
* package/runtime manager,
* service manager,
* networking stack,
* media framework,
* and application platform.

# 2. Runtime-Centric Instead of Process-Centric

Applications are NOT traditional heavyweight OS processes.

Applications run as:

* runtime cells,
* isolated execution compartments,
* namespace-bound environments,
* capability-scoped services.

Each application should:

* start rapidly,
* suspend safely,
* restore instantly,
* remain isolated,
* and survive service failures gracefully.

# 3. Inferno-Inspired but Independent

The Inferno source tree may be used for:

* runtime concepts,
* namespace models,
* distributed service topology,
* desktop/session ideas,
* IPC/service routing,
* lightweight orchestration,
* shell concepts,
* and filesystem mediation.

However:

* Ainux must become its OWN operating system,
* not a clone or fork of Inferno.

# 4. Full Desktop Environment

The desktop is now a first-class engineering target.

Required desktop components:

* graphics compositor,
* window manager,
* desktop shell,
* application launcher,
* terminal,
* task manager,
* runtime monitor,
* settings system,
* file explorer,
* notification service,
* media/audio services,
* software/runtime center.

Desktop services must remain:

* isolated,
* restartable,
* modular,
* namespace-aware,
* and capability-mediated.

# 5. Independent Runtime Ecosystem

Ainux applications should use:

* sovereign runtime APIs,
* capability-mediated services,
* execution-cell architecture,
* runtime namespaces,
* and service graph mediation.

The goal is to establish:

* a native Ainux software ecosystem,
* not dependence on Linux compatibility.

# 6. Kernel Philosophy

The kernel remains:

* minimal,
* service-oriented,
* and mediation-focused.

The kernel only handles:

* memory,
* scheduler,
* IPC primitives,
* capability tables,
* namespace attachment,
* runtime spawning,
* and object mediation.

Complexity moves outward into isolated runtime services.

# 7. No Heavy Hypervisor Dependency

The system must function WITHOUT:

* VT-x,
* AMD-V,
* nested virtualization,
* or heavyweight hypervisors.

Isolation must come from:

* capability mediation,
* runtime sandboxing,
* namespace isolation,
* memory segmentation,
* and userspace service boundaries.

# 8. Survivability-Oriented Memory Model

Implement:

* dormant runtime cells,
* compressed memory,
* swap-backed restoration,
* suspended execution states,
* and low-memory survivability.

The system should remain operational even under:

* severe memory pressure,
* partial runtime failures,
* or degraded hardware conditions.

# 9. Service-Oriented Core

Core runtime services:

* procd
* fsd
* netd
* gfxd
* wind
* audiod
* inputd
* deskd
* notifd
* runtimed

These services form the sovereign runtime graph.

# 10. Visual & Interaction Philosophy

The desktop should prioritize:

* responsiveness,
* clarity,
* low overhead,
* recoverability,
* and runtime observability.

Avoid:

* giant monolithic frameworks,
* unstable GPU dependence,
* excessive animation complexity,
* or oversized desktop stacks.

# Immediate Development Objectives

## Phase A — Stable Desktop Boot

Build:

* reliable boot,
* framebuffer,
* runtime initialization,
* userspace launch,
* desktop startup,
* stable logging.

## Phase B — Runtime Desktop Core

Build:

* graphics service,
* window manager,
* desktop shell,
* terminal,
* launcher,
* task manager,
* namespace explorer.

## Phase C — Execution Cell Platform

Build:

* runtime cell manager,
* sandboxed applications,
* capability namespaces,
* service graph attachment,
* application suspension/resume.

## Phase D — Sovereign Runtime Ecosystem

Build:

* native runtime APIs,
* runtime package manager,
* software center,
* userspace services,
* developer SDK.

## Phase E — Distributed Runtime Architecture

Eventually support:

* runtime migration,
* distributed execution,
* remote namespaces,
* portable execution cells,
* sovereign distributed computing.

# Engineering Constraints

Every subsystem must:

* boot independently,
* fail gracefully,
* remain observable,
* recover safely,
* and degrade predictably.

Never violate:

* explicit authority,
* runtime isolation,
* namespace mediation,
* or service modularity.

# Final Directive

Ainux is now officially transitioning into:

“A complete sovereign desktop operating system built around runtime-oriented architecture, execution-cell isolation, namespace mediation, and capability-secure service topology.”

This transformation supersedes the previous experimental kernel-only direction.

All future engineering work must align with this operating system vision.
