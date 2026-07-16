import type { MediaDetails } from "$lib/ipc/types";

export type DiscoveryCoverMotif =
  "crane" | "shrine" | "comet" | "archive" | "tide" | "bento";

export interface DiscoveryPreviewAnime {
  media: MediaDetails;
  catalogCode: string;
  motif: DiscoveryCoverMotif;
  matchSignals: readonly string[];
}

export const discoveryPreviewAnime = [
  {
    media: {
      anilistId: 980_081,
      title: "Paper Cranes at Dawn",
      coverUrl: null,
      mediaFormat: "TV",
      status: "RELEASING",
      episodes: 12,
      genres: ["Drama", "Slice of Life"],
      tags: [
        { name: "Found Family", rank: 88 },
        { name: "Rural", rank: 76 },
        { name: "Letters", rank: 71 },
      ],
      trending: 0,
      sequels: [],
    },
    catalogCode: "CAT-081",
    motif: "crane",
    matchSignals: ["found family", "quiet drama"],
  },
  {
    media: {
      anilistId: 980_144,
      title: "Neon Shrine Maintenance Club",
      coverUrl: null,
      mediaFormat: "TV_SHORT",
      status: "NOT_YET_RELEASED",
      episodes: null,
      genres: ["Comedy", "Supernatural"],
      tags: [
        { name: "Urban Fantasy", rank: 84 },
        { name: "School Club", rank: 75 },
        { name: "Cats", rank: 69 },
      ],
      trending: 0,
      sequels: [],
    },
    catalogCode: "CAT-144",
    motif: "shrine",
    matchSignals: ["stray cats", "urban fantasy"],
  },
  {
    media: {
      anilistId: 980_233,
      title: "How to Befriend a Comet",
      coverUrl: null,
      mediaFormat: "MOVIE",
      status: "FINISHED",
      episodes: 1,
      genres: ["Adventure", "Fantasy"],
      tags: [
        { name: "Space", rank: 91 },
        { name: "Coming of Age", rank: 79 },
        { name: "Travel", rank: 73 },
      ],
      trending: 0,
      sequels: [
        {
          anilistId: 990_233,
          title: "How to Greet the Morning Star",
          releaseDate: "2027-01-09",
        },
      ],
    },
    catalogCode: "CAT-233",
    motif: "comet",
    matchSignals: ["soft adventure", "space"],
  },
  {
    media: {
      anilistId: 980_377,
      title: "The Archive Beneath Rain",
      coverUrl: null,
      mediaFormat: "ONA",
      status: "RELEASING",
      episodes: 8,
      genres: ["Mystery", "Sci-Fi"],
      tags: [
        { name: "Archives", rank: 90 },
        { name: "Memory", rank: 86 },
        { name: "Cyberpunk", rank: 72 },
      ],
      trending: 0,
      sequels: [
        {
          anilistId: 990_377,
          title: "The Index Beyond Rain",
          releaseDate: null,
        },
      ],
    },
    catalogCode: "CAT-377",
    motif: "archive",
    matchSignals: ["memory mystery", "rainy night"],
  },
  {
    media: {
      anilistId: 980_418,
      title: "Saltwater Constellations",
      coverUrl: null,
      mediaFormat: "TV",
      status: "FINISHED",
      episodes: 13,
      genres: ["Romance", "Drama"],
      tags: [
        { name: "Coastal", rank: 87 },
        { name: "Astronomy", rank: 81 },
        { name: "Photography", rank: 74 },
      ],
      trending: 0,
      sequels: [],
    },
    catalogCode: "CAT-418",
    motif: "tide",
    matchSignals: ["coastal", "slow romance"],
  },
  {
    media: {
      anilistId: 980_512,
      title: "Midnight Bento Society",
      coverUrl: null,
      mediaFormat: "TV_SHORT",
      status: "RELEASING",
      episodes: 10,
      genres: ["Slice of Life", "Comedy"],
      tags: [
        { name: "Food", rank: 89 },
        { name: "Workplace", rank: 78 },
        { name: "Friendship", rank: 72 },
      ],
      trending: 0,
      sequels: [],
    },
    catalogCode: "CAT-512",
    motif: "bento",
    matchSignals: ["food", "late-night comfort"],
  },
] as const satisfies readonly DiscoveryPreviewAnime[];

export function matchesDiscoveryQuery(
  anime: DiscoveryPreviewAnime,
  query: string,
): boolean {
  const searchableText = [
    anime.media.title,
    anime.media.mediaFormat ?? "",
    ...anime.media.genres,
    ...anime.media.tags.map((tag) => tag.name),
  ]
    .join(" ")
    .toLocaleLowerCase();

  return searchableText.includes(query.trim().toLocaleLowerCase());
}
