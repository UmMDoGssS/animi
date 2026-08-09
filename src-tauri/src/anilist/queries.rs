pub const SEARCH_MEDIA: &str = r#"
query SearchMedia(
  $mediaType: MediaType!
  $includeAnimeFields: Boolean!
  $includeMangaFields: Boolean!
  $search: String!
  $page: Int!
  $perPage: Int!
) {
  Page(page: $page, perPage: $perPage) {
    pageInfo { hasNextPage }
    media(type: $mediaType, search: $search, isAdult: false, sort: [SEARCH_MATCH, POPULARITY_DESC]) {
      id
      type
      title { romaji english native }
      coverImage { large }
      format
      status
      episodes @include(if: $includeAnimeFields)
      duration @include(if: $includeAnimeFields)
      chapters @include(if: $includeMangaFields)
      volumes @include(if: $includeMangaFields)
      startDate { year month day }
      genres
      tags { name rank isAdult }
      trending
    }
  }
}
"#;

pub const TRENDING_MEDIA: &str = r#"
query TrendingMedia(
  $mediaType: MediaType!
  $includeAnimeFields: Boolean!
  $includeMangaFields: Boolean!
  $page: Int!
  $perPage: Int!
) {
  Page(page: $page, perPage: $perPage) {
    pageInfo { hasNextPage }
    media(type: $mediaType, isAdult: false, sort: [TRENDING_DESC, POPULARITY_DESC]) {
      id
      type
      title { romaji english native }
      coverImage { large }
      format
      status
      episodes @include(if: $includeAnimeFields)
      duration @include(if: $includeAnimeFields)
      chapters @include(if: $includeMangaFields)
      volumes @include(if: $includeMangaFields)
      startDate { year month day }
      genres
      tags { name rank isAdult }
      trending
    }
  }
}
"#;

pub const LIBRARY_MEDIA: &str = r#"
query LibraryMedia(
  $mediaType: MediaType!
  $includeAnimeFields: Boolean!
  $includeMangaFields: Boolean!
  $ids: [Int]
  $perPage: Int!
) {
  Page(page: 1, perPage: $perPage) {
    pageInfo { hasNextPage }
    media(type: $mediaType, id_in: $ids, isAdult: false) {
      id
      type
      title { romaji english native }
      coverImage { large }
      format
      status
      episodes @include(if: $includeAnimeFields)
      duration @include(if: $includeAnimeFields)
      chapters @include(if: $includeMangaFields)
      volumes @include(if: $includeMangaFields)
      startDate { year month day }
      genres
      tags { name rank isAdult }
      trending
    }
  }
}
"#;

pub const MEDIA_DETAILS: &str = r#"
query MediaDetails(
  $id: Int!
  $mediaType: MediaType!
  $includeAnimeFields: Boolean!
  $includeMangaFields: Boolean!
) {
  Media(id: $id, type: $mediaType, isAdult: false) {
    id
    type
    title { romaji english native }
    coverImage { large }
    format
    status
    episodes @include(if: $includeAnimeFields)
    nextAiringEpisode @include(if: $includeAnimeFields) { episode }
    duration @include(if: $includeAnimeFields)
    chapters @include(if: $includeMangaFields)
    volumes @include(if: $includeMangaFields)
    startDate { year month day }
    genres
    tags { name rank isAdult }
    trending
    relations {
      edges {
        relationType(version: 2)
        node {
          id
          type
          isAdult
          title { romaji english native }
          startDate { year month day }
        }
      }
    }
  }
}
"#;

pub const AIRING_SCHEDULE: &str = r#"
query AiringSchedule($mediaId: Int!, $page: Int!, $perPage: Int!) {
  Page(page: $page, perPage: $perPage) {
    pageInfo { hasNextPage }
    airingSchedules(mediaId: $mediaId, notYetAired: true, sort: TIME) {
      episode
      airingAt
    }
  }
}
"#;
