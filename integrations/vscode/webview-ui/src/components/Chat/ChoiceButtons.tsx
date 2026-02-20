/**
 * ChoiceButtons - Interactive A/B/C choice buttons
 * 
 * Detects options in Sentinel responses like:
 * - A) Option text
 * - B) Option text
 * - C) Option text
 * 
 * And renders them as clickable buttons.
 */

import React, { useMemo } from "react";
import { Button } from "../ui/button";
import { cn } from "@/lib/utils";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";

interface Choice {
  letter: string;
  text: string;
  fullText: string;
}

interface ChoiceButtonsProps {
  content: string;
  messageId: string;
  onChoiceSelected?: (choice: string) => void;
}

/**
 * Parse content for A/B/C options
 */
function parseChoices(content: string): Choice[] {
  const choices: Choice[] = [];
  
  // Split content into lines
  const lines = content.split('\n');
  let currentChoice: { letter: string; text: string } | null = null;
  let inChoiceSection = false;
  
  // Emoji pattern - matches common emoji used as section markers
  const emojiPattern = /^[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]/u;
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmedLine = line.trim();
    
    // Check if this line starts a new option: A), A., A -, B), etc.
    const optionMatch = trimmedLine.match(/^([A-C])[\).\s]+(.+)$/i);
    
    if (optionMatch) {
      inChoiceSection = true;
      
      // Save previous choice if exists
      if (currentChoice && currentChoice.text.trim().length > 0) {
        choices.push({
          letter: currentChoice.letter,
          text: currentChoice.text.trim(),
          fullText: `${currentChoice.letter}) ${currentChoice.text.trim()}`,
        });
      }
      
      // Start new choice
      currentChoice = {
        letter: optionMatch[1].toUpperCase(),
        text: optionMatch[2].trim(),
      };
    } else if (currentChoice && inChoiceSection) {
      // Check for stop conditions:
      // 1. Line starts with emoji (📋, 🎨, 🛡️, ❓, 🎯, etc.)
      // 2. Line starts with ##
      // 3. Empty line followed by non-choice content
      
      if (emojiPattern.test(trimmedLine) || trimmedLine.startsWith('##')) {
        // Save current and stop parsing choices
        if (currentChoice.text.trim().length > 0) {
          choices.push({
            letter: currentChoice.letter,
            text: currentChoice.text.trim(),
            fullText: `${currentChoice.letter}) ${currentChoice.text.trim()}`,
          });
        }
        currentChoice = null;
        inChoiceSection = false;
        break;
      }
      
      // If line is empty, check if next line is a new section or another choice
      if (trimmedLine.length === 0) {
        const nextLine = lines[i + 1]?.trim() || '';
        const nextIsChoice = /^[A-C][\).\s]/i.test(nextLine);
        const nextIsSection = emojiPattern.test(nextLine) || nextLine.startsWith('##');
        
        if (!nextIsChoice && !nextIsSection) {
          // End of choice section
          if (currentChoice.text.trim().length > 0) {
            choices.push({
              letter: currentChoice.letter,
              text: currentChoice.text.trim(),
              fullText: `${currentChoice.letter}) ${currentChoice.text.trim()}`,
            });
          }
          currentChoice = null;
          inChoiceSection = false;
          break;
        }
        continue;
      }
      
      // Check if this line looks like a new section (Pro:/Contro:/etc.)
      if (/^(Pro|Contro|Vantaggi|Svantaggi|Nota|Note):/i.test(trimmedLine)) {
        // This is part of the choice description, include it
        currentChoice!.text += ' ' + trimmedLine;
      }
    }
  }
  
  // Don't forget the last choice
  if (currentChoice && currentChoice.text.trim().length > 0) {
    choices.push({
      letter: currentChoice.letter,
      text: currentChoice.text.trim(),
      fullText: `${currentChoice.letter}) ${currentChoice.text.trim()}`,
    });
  }
  
  // Only return if we have at least 2 choices (A and B minimum)
  return choices.length >= 2 ? choices : [];
}

/**
 * Check if content contains choice options
 */
export function hasChoices(content: string): boolean {
  return parseChoices(content).length >= 2;
}

/**
 * Extract the question part before choices
 */
export function extractQuestionBeforeChoices(content: string): string {
  const choices = parseChoices(content);
  if (choices.length === 0) return content;
  
  // Find where the first choice starts
  const firstChoicePattern = /(?:^|\n)\s*[A-C][\).\s]/i;
  const match = firstChoicePattern.exec(content);
  
  if (match && match.index > 0) {
    return content.slice(0, match.index).trim();
  }
  
  return content;
}

export default function ChoiceButtons({ content, messageId, onChoiceSelected }: ChoiceButtonsProps) {
  const vscode = useVSCodeAPI();
  const choices = useMemo(() => parseChoices(content), [content]);
  
  // No choices found or not enough choices
  if (choices.length < 2) {
    return null;
  }
  
  const handleChoice = (choice: Choice) => {
    // Send the choice as a user message
    const choiceText = choice.text;
    
    vscode.postMessage({
      type: "chatMessage",
      text: choiceText,
    });
    
    // Callback if provided
    onChoiceSelected?.(choiceText);
  };
  
  return (
    <div className="flex flex-col gap-2 mt-3 pt-3 border-t border-border/50">
      <span className="text-xs text-muted-foreground font-medium">Scegli un'opzione:</span>
      <div className="flex flex-col gap-2">
        {choices.map((choice) => (
          <Button
            key={choice.letter}
            variant="outline"
            size="sm"
            onClick={() => handleChoice(choice)}
            className={cn(
              "justify-start h-auto py-2.5 px-3 text-left",
              "hover:bg-primary/10 hover:border-primary/30 hover:text-primary",
              "transition-all duration-200"
            )}
          >
            <span className="flex items-start gap-2 w-full">
              <span className="flex-shrink-0 size-5 rounded-full bg-primary/10 text-primary text-xs font-semibold flex items-center justify-center">
                {choice.letter}
              </span>
              <span className="flex-1 text-sm leading-relaxed">
                {choice.text}
              </span>
            </span>
          </Button>
        ))}
      </div>
    </div>
  );
}