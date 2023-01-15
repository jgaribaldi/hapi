# Event driven migration checklist

## Create project structure
- [x] Modules
- [x] Interfaces
- [x] Commands and Events

## Refactor Modules
- [x] Core
- [] Probes
- [] Stats

## Create infrastructure that supports event driven
- [] Should receive a command and transform it into a call to the app's core
- [] Should emit an event based on the function's result (write it to the event bus?)
- [] Create event bus (broadcast channel)
- [] Create command input for each module (mpsc channel?)

## Refactor Interfaces
- [] API

## Create app settings
- [] App's IP address and port
- [] API's IP address and port
- [] Probe default configuration

## Create app database
- [] JSON file with routes

## 'Core' Commands
- [] LookupUpstream
- [] EnableUpstream
- [] DisableUpstream
- [] AddRoute
- [] RemoveRoute

## 'Probe' Commands
- [] StartProbe
- [] PauseProbe
- [] StopProbe

## 'Stats' Commands
- [] LookupStats

## 'Core' Events
- [] UpstreamWasFound
- [] UpstreamWasNotFound
- [] UpstreamWasEnabled
- [] UpstreamWasDisabled
- [] RouteWasAdded
- [] RouteWasNotAdded
- [] RouteWasRemoved
- [] RouteWasNotRemoved

## 'Probe' Events
- [] ProbeWasStarted
- [] ProbeWasPaused
- [] ProbeWasStopped

## 'Stats' Events
- [] StatsWereFound
- [] StatsWereNotFound