# Clique Fusion

[![Latest Docs](https://docs.rs/clique-fusion/badge.svg)](https://docs.rs/clique-fusion/)
[![Continuous integration](https://github.com/danieleades/clique-fusion/actions/workflows/CI.yml/badge.svg)](https://github.com/danieleades/clique-fusion/actions/workflows/CI.yml)
[![codecov](https://codecov.io/gh/danieleades/clique-fusion/graph/badge.svg?token=4laxho1ik5)](https://codecov.io/gh/danieleades/clique-fusion)

The following library provides an efficient algorithm for 'fusing' multiple observations of objects.

An 'observation' is a discrete measurement of the position of an object, with associated uncertainty.

A single object may be supported by multiple observations. These may be taken by different sensors, or by different 'passes' with a single sensor, or may be taken at different times.

Due to positional uncertainty, two observations of the same object may have different apparent positions.

This library ingests a list of observations and groups clusters of nearby observations when they are close enough that we can assume they are observations of the same underlying object.

For a cluster of nearby observations to be grouped, they must all mutually include each other within their respective error bounds. This prevents, for example, a 'line' of observations being grouped. It also prevents two observations from being grouped when one has a very large error bound, but the other has a more precisely known (and inconsistent) position.

## Algorithm

The algorithm uses a graph-based approach with clique detection to ensure mutual inclusion constraints are satisfied

1. Build a spatial index of observations
2. for each observation, find all neighbours within the observation's error ellipse
3. for each neighbour, check that the original observation lies with the neighbour's error ellipse
4. generate a graph of 'mutually compatible' observations
5. find maximal cliques of the compatibility graph

## API

This library supports finding cliques from a vector of observations (fast), and for adding observations one at a time (slower, but supports real-time or incremental use cases).

---

## Licensing

This project is publicly available under the GNU General Public License v3.0. It may optionally be distributed under the permissive MIT license by commercial arrangement.
