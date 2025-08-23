// <copyright file="Clique.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion
{
    using System;
    using System.Collections.Generic;

    /// <summary>
    /// A maximal clique of mutually compatible observations.
    /// </summary>
    public record Clique(IReadOnlyList<Guid> ObservationIds);
}
