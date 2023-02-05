# Event driven migration checklist

## Create project structure
- [x] Modules
- [x] Interfaces
- [x] Commands and Events

## Refactor Modules
- [x] Core
- [x] Stats
- [x] Probes

## Create infrastructure that supports event driven
- [x] Should receive a command and transform it into a call to the app's core
- [x] Should emit an event based on the function's result (write it to the event bus?)
- [x] Create event bus (broadcast channel)
- [x] Create command input for each module (mpsc channel?)

## Refactor app's process structure
- [x] Create appropriate processes and launch them from main
- [x] Perform upstream lookup through commands and events
- [x] Perform upstream enable and disable through commands and events
- [] Remove old process structure

## Extract probe handler
- [x] Create a separate file for it
- [x] Refactor it, so it only reacts to certain events, as opposed to receive commands

## Refactor Interfaces
- [] API

## Abstractions
- [x] Create Core client to abstract send commands and listen to events

## APIs
- [x] Routes API
- [x] Upstreams API
- [x] Stats API

## Stats
- [] Count stats by listening to upstream lookup events

## Create app settings
- [] App's IP address and port
- [] API's IP address and port
- [] Probe default configuration

## Create app database
- [] JSON file with routes

## 'Core' Commands
- [x] LookupUpstream
- [x] EnableUpstream
- [x] DisableUpstream
- [x] AddRoute
- [x] RemoveRoute

## 'Probe' Commands
- [] StartProbe
- [] StopProbe

## 'Stats' Commands
- [] CountStat
- [] LookupStats

## 'Core' Events
- [x] UpstreamWasFound
- [x] UpstreamWasNotFound
- [x] UpstreamWasEnabled
- [x] UpstreamWasDisabled
- [x] RouteWasAdded
- [x] RouteWasNotAdded
- [x] RouteWasRemoved
- [x] RouteWasNotRemoved

## 'Probe' Events
- [] ProbeWasStarted
- [] ProbeWasStopped

## 'Stats' Events
- [] StatWasCounted
- [] StatsWereFound
- [] StatsWereNotFound