import { invoke } from "@tauri-apps/api/core";

export interface NewsArticle {
  uuid: string;
  title: string;
  snippet: string;
  url: string;
  source: string;
  published_at: string;
  entities: NewsEntity[];
  sentiment: number | null;
  image_url: string | null;
}

export interface NewsEntity {
  symbol: string;
  name: string;
  sentiment_score: number | null;
  industry: string | null;
  country: string | null;
}

export async function fetchNews(params: {
  symbols?: string;
  search?: string;
  minSentiment?: number;
  maxSentiment?: number;
}): Promise<NewsArticle[]> {
  return invoke<NewsArticle[]>("news_fetch", {
    symbols: params.symbols ?? null,
    search: params.search ?? null,
    minSentiment: params.minSentiment ?? null,
    maxSentiment: params.maxSentiment ?? null,
  });
}

export async function testKey(): Promise<boolean> {
  return invoke<boolean>("news_test_key");
}

// ---------- Calendar ----------

export interface CalendarEvent {
  title: string;
  country: string;
  currency: string;
  date: string;
  impact: "High" | "Medium" | "Low" | "Holiday";
  forecast: string;
  previous: string;
}

export async function fetchCalendar(): Promise<CalendarEvent[]> {
  return invoke<CalendarEvent[]>("calendar_fetch");
}

// ---------- FMP ----------

export interface FmpEvent {
  title: string;
  country: string;
  currency: string;
  date: string;
  impact: string;
  forecast: string;
  previous: string;
  actual: string;
  description: string;
}

export async function fetchFmpCalendar(from: string, to: string): Promise<FmpEvent[]> {
  return invoke<FmpEvent[]>("fmp_calendar_fetch", { from, to });
}

// ---------- Finnhub ----------

export interface FinnhubArticle {
  id: number;
  category: string;
  datetime: number;
  headline: string;
  image: string;
  related: string;
  source: string;
  summary: string;
  url: string;
}

export async function fetchFinnhubNews(): Promise<FinnhubArticle[]> {
  return invoke<FinnhubArticle[]>("finnhub_news");
}
