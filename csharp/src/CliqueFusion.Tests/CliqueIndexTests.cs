using System;
using System.Collections.Generic;
using System.Linq;
using Xunit;

namespace CliqueFusion.Tests
{
    /// <summary>
    /// Tests for the high-level CliqueIndex wrapper around the native FFI.
    /// These tests ensure correct interop and fusion logic based on context rules.
    /// </summary>
    public class CliqueIndexTests
    {
        private const double Chi2Threshold = 5.99;

        private static Observation CreateObservation(double x, double y, Guid? context = null)
        {
            return new Observation(Guid.NewGuid(), x, y, 1.0, 0.0, 1.0, context);
        }

        [Fact]
        public void CanCreateEmptyIndex()
        {
            using var index = new CliqueIndex(chi2Threshold: CliqueThresholds.Confidence95);
            var cliques = index.GetCliques();
            Assert.Empty(cliques);
        }

        [Fact]
        public void SingleObservationDoesNotFormClique()
        {
            using var index = new CliqueIndex(CliqueThresholds.Confidence95);
            index.Insert(CreateObservation(1.0, 2.0, context: null));
            Assert.Empty(index.GetCliques());
        }

        [Fact]
        public void ObservationsWithSameContextDoNotFuse()
        {
            var sharedContext = Guid.Parse("cdc2943c-4801-428a-8f55-eee2019b1db5");
            var obs1 = CreateObservation(1.0, 2.0, sharedContext);
            var obs2 = CreateObservation(1.1, 2.1, sharedContext);

            using var index = new CliqueIndex(30.0);
            index.Insert(obs1);
            index.Insert(obs2);

            // Even though positions are close, context match suppresses fusion
            Assert.Empty(index.GetCliques());
        }

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

        [Fact]
        public void ConstructorAcceptsInitialObservationsAndFuses()
        {
            var obs1 = CreateObservation(1.0, 2.0, null);
            var obs2 = CreateObservation(1.1, 2.1, null);

            using var index = new CliqueIndex(new List<Observation> { obs1, obs2 }, 30.0);
            var cliques = index.GetCliques();

            Assert.Single(cliques);
        }

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

        [Fact]
        public void DisposingIndexPreventsUsage()
        {
            var index = new CliqueIndex(Chi2Threshold);
            index.Dispose();

            Assert.Throws<ObjectDisposedException>(() => index.Insert(CreateObservation(0, 0)));
            Assert.Throws<ObjectDisposedException>(() => index.GetCliques());
        }

        [Fact]
        public void CreatingWithNullObservationListThrows()
        {
            Assert.Throws<ArgumentNullException>(() => new CliqueIndex(null!, CliqueThresholds.Confidence95));
        }
    }
}
