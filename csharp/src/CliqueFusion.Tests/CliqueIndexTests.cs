// <copyright file="CliqueIndexTests.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion.Tests
{
    using Xunit;

    /// <summary>
    /// Tests for the high-level CliqueIndex wrapper around the native FFI.
    /// These tests ensure correct interop and fusion logic based on context rules.
    /// </summary>
    public class CliqueIndexTests
    {
        private const double Chi2Threshold = 5.99;

        /// <summary>
        /// Verifies that a new CliqueIndex contains no cliques.
        /// </summary>
        [Fact]
        public void CanCreateEmptyIndex()
        {
            using var index = new CliqueIndex(chi2Threshold: CliqueThresholds.Confidence95);
            var cliques = index.GetCliques();
            Assert.Empty(cliques);
        }

        /// <summary>
        /// Verifies that a single observation without context does not form a clique.
        /// </summary>
        [Fact]
        public void SingleObservationDoesNotFormClique()
        {
            using var index = new CliqueIndex(CliqueThresholds.Confidence95);
            index.Insert(CreateObservation(1.0, 2.0, context: null));
            Assert.Empty(index.GetCliques());
        }

        /// <summary>
        /// Verifies that observations with matching context IDs do not fuse into a clique.
        /// </summary>
        [Fact]
        public void ObservationsWithSameContextDoNotFuse()
        {
            var sharedContext = Guid.Parse("cdc2943c-4801-428a-8f55-eee2019b1db5");
            var obs1 = CreateObservation(1.0, 2.0, sharedContext);
            var obs2 = CreateObservation(1.1, 2.1, sharedContext);

            using var index = new CliqueIndex(30.0);
            index.Insert(obs1);
            index.Insert(obs2);

            Assert.Empty(index.GetCliques());
        }

        /// <summary>
        /// Verifies that observations with different context IDs can fuse into a clique if spatially close.
        /// </summary>
        [Fact]
        public void ObservationsWithDifferentContextsCanFuse()
        {
            var obs1 = CreateObservation(1.0, 2.0, Guid.Parse("c617b774-c88b-4456-b9ec-cafc6df46e01"));
            var obs2 = CreateObservation(1.1, 2.1, Guid.Parse("122ac28b-5a70-4d3a-b538-68596a000974"));

            using var index = new CliqueIndex(30.0);
            index.Insert(obs1);
            index.Insert(obs2);

            var cliques = index.GetCliques();
            Assert.Single(cliques);
            var ids = cliques[0].ObservationIds;
            Assert.Contains(obs1.Id, ids);
            Assert.Contains(obs2.Id, ids);
        }

        /// <summary>
        /// Verifies that observations with null contexts can fuse if spatially close.
        /// </summary>
        [Fact]
        public void ObservationsWithNullContextsCanFuse()
        {
            var obs1 = CreateObservation(1.0, 2.0, null);
            var obs2 = CreateObservation(1.1, 2.1, null);

            using var index = new CliqueIndex(30.0);
            index.Insert(obs1);
            index.Insert(obs2);

            var cliques = index.GetCliques();
            Assert.Single(cliques);
            var ids = cliques[0].ObservationIds;
            Assert.Contains(obs1.Id, ids);
            Assert.Contains(obs2.Id, ids);
        }

        /// <summary>
        /// Verifies that observations with mixed null and non-null contexts can still fuse.
        /// </summary>
        [Fact]
        public void ObservationsWithNullAndNonNullContextCanFuse()
        {
            var obs1 = CreateObservation(1.0, 2.0, null);
            var obs2 = CreateObservation(1.1, 2.1, Guid.NewGuid());

            using var index = new CliqueIndex(30.0);
            index.Insert(obs1);
            index.Insert(obs2);

            var cliques = index.GetCliques();
            Assert.Single(cliques);
        }

        /// <summary>
        /// Verifies that CliqueIndex can fuse observations passed to its constructor.
        /// </summary>
        [Fact]
        public void ConstructorAcceptsInitialObservationsAndFuses()
        {
            var obs1 = CreateObservation(1.0, 2.0, null);
            var obs2 = CreateObservation(1.1, 2.1, null);

            using var index = new CliqueIndex(new List<Observation> { obs1, obs2 }, 30.0);
            var cliques = index.GetCliques();

            Assert.Single(cliques);
        }

        /// <summary>
        /// Verifies that inserting observations after construction is cumulative and fusion-aware.
        /// </summary>
        [Fact]
        public void InsertingAfterConstructionAccumulates()
        {
            var obs1 = CreateObservation(1.0, 2.0, null);
            var obs2 = CreateObservation(1.1, 2.1, null);

            using var index = new CliqueIndex(new List<Observation> { obs1 }, 30.0);
            index.Insert(obs2);

            var cliques = index.GetCliques();
            Assert.Single(cliques);
            Assert.Contains(cliques[0].ObservationIds, id => id == obs1.Id);
            Assert.Contains(cliques[0].ObservationIds, id => id == obs2.Id);
        }

        /// <summary>
        /// Verifies that using a disposed index throws appropriate exceptions.
        /// </summary>
        [Fact]
        public void DisposingIndexPreventsUsage()
        {
            var index = new CliqueIndex(Chi2Threshold);
            index.Dispose();

            Assert.Throws<ObjectDisposedException>(() => index.Insert(CreateObservation(0, 0)));
            Assert.Throws<ObjectDisposedException>(() => index.GetCliques());
        }

        /// <summary>
        /// Verifies that passing null as the initial observation list throws an ArgumentNullException.
        /// </summary>
        [Fact]
        public void CreatingWithNullObservationListThrows()
        {
            Assert.Throws<ArgumentNullException>(() => new CliqueIndex(null!, CliqueThresholds.Confidence95));
        }

        /// <summary>
        /// Creates a test Observation with fixed covariance and optional context.
        /// </summary>
        private static Observation CreateObservation(double x, double y, Guid? context = null)
        {
            return new Observation(Guid.NewGuid(), x, y, 1.0, 0.0, 1.0, context);
        }
    }
}
