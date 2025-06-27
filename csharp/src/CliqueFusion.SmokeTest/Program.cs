using CliqueFusion;

Console.WriteLine("🔥 Running CliqueFusion smoke test...");

var obs1 = new Observation(Guid.NewGuid(), 1.0, 2.0, 1.0, 0.0, 1.0);
var obs2 = new Observation(Guid.NewGuid(), 1.1, 2.1, 1.0, 0.0, 1.0);
var obs3 = new Observation(Guid.NewGuid(), 5.0, 5.0, 1.0, 0.0, 1.0);

using var index = new CliqueIndex(new[] { obs1, obs2, obs3 }, CliqueThresholds.Confidence95);

var cliques = index.GetCliques();
Console.WriteLine($"✅ Found {cliques.Count} cliques.");

foreach (var clique in cliques)
{
    Console.WriteLine($"Clique ({clique.ObservationIds.Count} observations):");
    foreach (var id in clique.ObservationIds)
        Console.WriteLine($" - {id}");
}
