//! DOCX Document Reader
//! Reads and parses Microsoft Word documents for trading strategy analysis

use anyhow::{Result, anyhow};
use log::{info, warn, debug};
use std::path::Path;
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

/// DOCX document content
#[derive(Debug, Clone)]
pub struct DocxContent {
    pub text: String,
    pub paragraphs: Vec<String>,
    pub tables: Vec<DocxTable>,
    pub metadata: DocxMetadata,
}

/// Table extracted from DOCX
#[derive(Debug, Clone)]
pub struct DocxTable {
    pub rows: Vec<Vec<String>>,
    pub headers: Option<Vec<String>>,
}

/// Document metadata
#[derive(Debug, Clone)]
pub struct DocxMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub word_count: usize,
}

/// DOCX Reader with trading strategy focus
pub struct DocxReader {
    extract_tables: bool,
    extract_metadata: bool,
    min_paragraph_length: usize,
}

impl DocxReader {
    /// Create new DOCX reader with default settings
    pub fn new() -> Self {
        Self {
            extract_tables: true,
            extract_metadata: true,
            min_paragraph_length: 10,
        }
    }
    
    /// Create DOCX reader for trading strategy documents
    pub fn for_trading_strategy() -> Self {
        Self {
            extract_tables: true,  // Tables often contain trading data
            extract_metadata: true,
            min_paragraph_length: 20, // Longer paragraphs for strategy descriptions
        }
    }
    
    /// Configure table extraction
    pub fn with_tables(mut self, extract: bool) -> Self {
        self.extract_tables = extract;
        self
    }
    
    /// Configure metadata extraction
    pub fn with_metadata(mut self, extract: bool) -> Self {
        self.extract_metadata = extract;
        self
    }
    
    /// Set minimum paragraph length
    pub fn with_min_paragraph_length(mut self, length: usize) -> Self {
        self.min_paragraph_length = length;
        self
    }
    
    /// Read DOCX file and extract content
    pub async fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<DocxContent> {
        let path = path.as_ref();
        info!("📄 Reading DOCX file: {}", path.display());
        
        // Validate file exists and has correct extension
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", path.display()));
        }
        
        if !path.extension().map_or(false, |ext| ext == "docx") {
            warn!("⚠️ File doesn't have .docx extension: {}", path.display());
        }
        
        // Open and read the DOCX file
        let file = File::open(path)
            .map_err(|e| anyhow!("Failed to open file {}: {}", path.display(), e))?;
        
        let mut archive = ZipArchive::new(file)
            .map_err(|e| anyhow!("Failed to read DOCX archive: {}", e))?;
        
        // Extract document content
        let text = self.extract_document_text(&mut archive)?;
        let paragraphs = self.extract_paragraphs(&text);
        
        let tables = if self.extract_tables {
            self.extract_tables_from_archive(&mut archive)?
        } else {
            Vec::new()
        };
        
        let metadata = if self.extract_metadata {
            self.extract_metadata_from_archive(&mut archive)?
        } else {
            DocxMetadata::default()
        };
        
        info!("✅ Successfully read DOCX: {} paragraphs, {} tables", 
              paragraphs.len(), tables.len());
        
        Ok(DocxContent {
            text,
            paragraphs,
            tables,
            metadata,
        })
    }
    
    /// Extract main document text from document.xml
    fn extract_document_text(&self, archive: &mut ZipArchive<File>) -> Result<String> {
        debug!("Extracting document text from word/document.xml");
        
        let mut document_xml = archive.by_name("word/document.xml")
            .map_err(|e| anyhow!("Failed to find document.xml: {}", e))?;
        
        let mut xml_content = String::new();
        document_xml.read_to_string(&mut xml_content)
            .map_err(|e| anyhow!("Failed to read document.xml: {}", e))?;
        
        // Simple XML text extraction (removes tags)
        let text = self.extract_text_from_xml(&xml_content);
        
        debug!("Extracted {} characters of text", text.len());
        Ok(text)
    }
    
    /// Extract text from XML by removing tags
    fn extract_text_from_xml(&self, xml: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        let mut chars = xml.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => text.push(ch),
                _ => {}
            }
        }
        
        // Clean up whitespace
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Split text into meaningful paragraphs
    fn extract_paragraphs(&self, text: &str) -> Vec<String> {
        // Try multiple splitting strategies
        let mut paragraphs = Vec::new();

        // Strategy 1: Split by double newlines (standard paragraphs)
        for chunk in text.split("\n\n") {
            let trimmed = chunk.trim().to_string();
            if trimmed.len() >= self.min_paragraph_length {
                paragraphs.push(trimmed);
            }
        }

        // Strategy 2: If we got only one big paragraph, split by sentences
        if paragraphs.len() <= 1 && text.len() > 1000 {
            paragraphs.clear();
            for sentence in text.split(". ") {
                let trimmed = sentence.trim().to_string();
                if trimmed.len() >= self.min_paragraph_length {
                    paragraphs.push(trimmed);
                }
            }
        }

        // Strategy 3: If still too few, split by specific patterns
        if paragraphs.len() <= 3 && text.len() > 2000 {
            paragraphs.clear();
            // Split by common section markers
            let patterns = vec![
                "Wprowadzenie:",
                "Architektura:",
                "Implementacja:",
                "Konfiguracja:",
                "Bezpieczeństwo:",
                "Testowanie:",
                "Wnioski:",
                "Dalsze Kroki:",
            ];

            let mut current_text = text;
            for pattern in patterns {
                if let Some(pos) = current_text.find(pattern) {
                    if pos > self.min_paragraph_length {
                        let before = current_text[..pos].trim();
                        if before.len() >= self.min_paragraph_length {
                            paragraphs.push(before.to_string());
                        }
                    }
                    current_text = &current_text[pos..];
                }
            }

            // Add remaining text
            if current_text.len() >= self.min_paragraph_length {
                paragraphs.push(current_text.trim().to_string());
            }
        }

        // Fallback: If still no good paragraphs, chunk by size
        if paragraphs.is_empty() && text.len() > self.min_paragraph_length {
            let chunk_size = 500; // 500 characters per chunk
            for chunk in text.as_bytes().chunks(chunk_size) {
                if let Ok(chunk_str) = std::str::from_utf8(chunk) {
                    let trimmed = chunk_str.trim().to_string();
                    if trimmed.len() >= self.min_paragraph_length {
                        paragraphs.push(trimmed);
                    }
                }
            }
        }

        paragraphs
    }
    
    /// Extract tables from DOCX (simplified implementation)
    fn extract_tables_from_archive(&self, _archive: &mut ZipArchive<File>) -> Result<Vec<DocxTable>> {
        // Simplified table extraction - in real implementation would parse document.xml for tables
        debug!("Table extraction not fully implemented - returning empty tables");
        Ok(Vec::new())
    }
    
    /// Extract metadata from core.xml and app.xml
    fn extract_metadata_from_archive(&self, archive: &mut ZipArchive<File>) -> Result<DocxMetadata> {
        debug!("Extracting metadata from docProps/core.xml");
        
        // Try to read core properties
        let mut metadata = DocxMetadata::default();
        
        if let Ok(mut core_xml) = archive.by_name("docProps/core.xml") {
            let mut xml_content = String::new();
            if core_xml.read_to_string(&mut xml_content).is_ok() {
                // Simple metadata extraction (would need proper XML parsing in production)
                if let Some(title) = self.extract_xml_value(&xml_content, "dc:title") {
                    metadata.title = Some(title);
                }
                if let Some(author) = self.extract_xml_value(&xml_content, "dc:creator") {
                    metadata.author = Some(author);
                }
            }
        }
        
        Ok(metadata)
    }
    
    /// Extract value from XML tag (simplified)
    fn extract_xml_value(&self, xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);
        
        if let Some(start) = xml.find(&start_tag) {
            let content_start = start + start_tag.len();
            if let Some(end) = xml[content_start..].find(&end_tag) {
                let content = &xml[content_start..content_start + end];
                return Some(content.trim().to_string());
            }
        }
        None
    }
    
    /// Search for trading-related keywords in document
    pub fn find_trading_keywords(&self, content: &DocxContent) -> Vec<TradingKeyword> {
        let keywords = vec![
            "arbitrage", "profit", "loss", "strategy", "trading", "buy", "sell",
            "price", "volume", "liquidity", "slippage", "fee", "risk", "return",
            "SOL", "USDC", "Raydium", "Orca", "DEX", "swap", "pool", "token",
            "wallet", "ledger", "mainnet", "devnet", "transaction", "signature"
        ];
        
        let mut found_keywords = Vec::new();
        let text_lower = content.text.to_lowercase();
        
        for keyword in keywords {
            let count = text_lower.matches(keyword).count();
            if count > 0 {
                found_keywords.push(TradingKeyword {
                    keyword: keyword.to_string(),
                    count,
                    context: self.extract_keyword_context(&content.text, keyword),
                });
            }
        }
        
        found_keywords.sort_by(|a, b| b.count.cmp(&a.count));
        found_keywords
    }
    
    /// Extract context around keyword
    fn extract_keyword_context(&self, text: &str, keyword: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut contexts = Vec::new();
        
        for (i, word) in words.iter().enumerate() {
            if word.to_lowercase().contains(&keyword.to_lowercase()) {
                let start = i.saturating_sub(5);
                let end = (i + 6).min(words.len());
                let context = words[start..end].join(" ");
                contexts.push(context);
                
                if contexts.len() >= 3 { // Limit to 3 contexts per keyword
                    break;
                }
            }
        }
        
        contexts
    }
}

/// Trading keyword found in document
#[derive(Debug, Clone)]
pub struct TradingKeyword {
    pub keyword: String,
    pub count: usize,
    pub context: Vec<String>,
}

impl Default for DocxMetadata {
    fn default() -> Self {
        Self {
            title: None,
            author: None,
            created: None,
            modified: None,
            word_count: 0,
        }
    }
}

impl DocxContent {
    /// Get summary of document content
    pub fn summary(&self) -> String {
        format!(
            "DOCX Content Summary:\n\
            - Text length: {} characters\n\
            - Paragraphs: {}\n\
            - Tables: {}\n\
            - Title: {}\n\
            - Author: {}",
            self.text.len(),
            self.paragraphs.len(),
            self.tables.len(),
            self.metadata.title.as_deref().unwrap_or("Unknown"),
            self.metadata.author.as_deref().unwrap_or("Unknown")
        )
    }
    
    /// Search for specific text in content
    pub fn search(&self, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        self.paragraphs
            .iter()
            .filter(|p| p.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }
}
