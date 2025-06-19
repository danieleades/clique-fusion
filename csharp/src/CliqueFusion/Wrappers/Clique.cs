// <copyright file="Clique.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion
{
    using System;
    using System.Collections.Generic;

    /// <summary>
    /// Represents a maximal clique — a group of observation IDs that have been clustered together
    /// as mutually compatible under the chi-squared threshold.
    /// </summary>
    public record Clique(IReadOnlyList<Guid> ObservationIds);
}
