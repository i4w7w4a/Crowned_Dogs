import { nextFloat, seedRng } from "@crowned-dogs/core";

/** A repeatable Math.random for tests. */
export const seeded = (seed: string) => {
  const rng = seedRng(seed);
  return () => nextFloat(rng);
};
