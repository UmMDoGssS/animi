pub const SEARCH_MEDIA: &str = r#"
query SearchMedia($search: String!, $page: Int!, $perPage: Int!) {
  Page(page: $page, perPage: $perPage) {
    pageInfo { hasNextPage }
    media(type: ANIME, search: $search, isAdult: false, sort: [SEARCH_MATCH, POPULARITY_DESC]) {
      id
      title { romaji english native }
      coverImage { large }
      format
      status
      episodes
      duration
      startDate { year month day }
      genres
      tags { name rank isAdult }
      trending
    }
  }
}
"#;

pub const TRENDING_MEDIA: &str = r#"
query TrendingMedia($page: Int!, $perPage: Int!) {
  Page(page: $page, perPage: $perPage) {
    pageInfo { hasNextPage }
    media(type: ANIME, isAdult: false, sort: [TRENDING_DESC, POPULARITY_DESC]) {
      id
      title { romaji english native }
      coverImage { large }
      format
      status
      episodes
      duration
      startDate { year month day }
      genres
      tags { name rank isAdult }
      trending
    }
  }
}
"#;

pub const LIBRARY_MEDIA: &str = r#"
query LibraryMedia($ids: [Int], $perPage: Int!) {
  Page(page: 1, perPage: $perPage) {
    pageInfo { hasNextPage }
    media(type: ANIME, id_in: $ids, isAdult: false) {
      id
      title { romaji english native }
      coverImage { large }
      format
      status
      episodes
      duration
      startDate { year month day }
      genres
      tags { name rank isAdult }
      trending
    }
  }
}
"#;

pub const MEDIA_DETAILS: &str = r#"
query MediaDetails($id: Int!) {
  Media(id: $id, type: ANIME, isAdult: false) {
    id
    title { romaji english native }
    coverImage { large }
    format
    status
    episodes
    duration
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
