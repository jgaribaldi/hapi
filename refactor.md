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

## Refactor Interfaces
- [] API

## Create app settings
- [] App's IP address and port
- [] API's IP address and port
- [] Probe default configuration

## Create app database
- [] JSON file with routes

## 'Core' Commands
- [x] LookupUpstream
- [] EnableUpstream
- [] DisableUpstream
- [x] AddRoute
- [] RemoveRoute

## 'Probe' Commands
- [] StartProbe
- [] PauseProbe
- [] StopProbe

## 'Stats' Commands
- [] CountStat
- [] LookupStats

## 'Core' Events
- [x] UpstreamWasFound
- [x] UpstreamWasNotFound
- [] UpstreamWasEnabled
- [] UpstreamWasDisabled
- [x] RouteWasAdded
- [x] RouteWasNotAdded
- [] RouteWasRemoved
- [] RouteWasNotRemoved

## 'Probe' Events
- [] ProbeWasStarted
- [] ProbeWasPaused
- [] ProbeWasStopped

## 'Stats' Events
- [] StatWasCounted
- [] StatsWereFound
- [] StatsWereNotFound