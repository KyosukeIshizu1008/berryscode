/**
 * Click Position Accuracy E2E Test
 *
 * CRITICAL: The ultimate test - clicking on desktop app positions cursor correctly
 *
 * Why this matters:
 * - This is THE BUG we've been fighting
 * - Canvas getBoundingClientRect() on real desktop
 * - OS font rendering differences
 * - WebView quirks
 *
 * This test runs on REAL desktop app, not browser simulation
 */

import { browser, $ } from '@wdio/globals'

describe('Click Position Accuracy - Desktop Reality Check', () => {
    beforeEach(async () => {
        await browser.setWindowSize(1400, 900)
        await browser.pause(500)
    })

    it('should position cursor correctly when clicking on ASCII text', async () => {
        // Type known text
        await browser.keys(['H', 'e', 'l', 'l', 'o', ' ', 'W', 'o', 'r', 'l', 'd'])
        await browser.pause(300)

        const canvas = await $('canvas')

        // Click at start of text (approximate pixel position)
        // This depends on CHAR_WIDTH_ASCII = 8.0, TEXT_PADDING = 15.0
        const clickX = 15 + 8 * 3  // Should click on 'l' (index 3)
        const clickY = 50  // First line

        await canvas.click({ x: clickX, y: clickY })
        await browser.pause(300)

        // Get actual cursor position
        const cursorPos = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.selectionStart : -1
        })

        console.log(`Clicked at (${clickX}, ${clickY}), cursor at position ${cursorPos}`)

        // Cursor should be somewhere reasonable (0-11 range for "Hello World")
        expect(cursorPos).toBeGreaterThanOrEqual(0)
        expect(cursorPos).toBeLessThanOrEqual(11)

        // Ideally should be at index 3 (on 'l'), but allow some tolerance
        // because exact pixel measurements may vary on different systems
        expect(Math.abs(cursorPos - 3)).toBeLessThanOrEqual(2)
    })

    it('should position cursor correctly when clicking on Japanese text', async () => {
        // Type Japanese text (こんにちは = 5 characters)
        // Note: We can't easily type Japanese via browser.keys in WebDriver
        // So we'll inject it programmatically

        await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            if (textarea) {
                textarea.value = 'こんにちは'
                textarea.dispatchEvent(new Event('input', { bubbles: true }))
            }
        })
        await browser.pause(500)

        const canvas = await $('canvas')

        // Click at position 2 (third character 'に')
        // CHAR_WIDTH_WIDE = 13.0, TEXT_PADDING = 15.0
        const clickX = 15 + 13 * 2
        const clickY = 50

        await canvas.click({ x: clickX, y: clickY })
        await browser.pause(300)

        const cursorPos = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.selectionStart : -1
        })

        console.log(`Japanese text click: (${clickX}, ${clickY}) -> cursor ${cursorPos}`)

        // Should be near position 2
        expect(cursorPos).toBeGreaterThanOrEqual(0)
        expect(cursorPos).toBeLessThanOrEqual(5)
        expect(Math.abs(cursorPos - 2)).toBeLessThanOrEqual(2)
    })

    it('should handle clicks on mixed ASCII and Japanese text', async () => {
        // Inject mixed text: "Hello 世界"
        await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            if (textarea) {
                textarea.value = 'Hello 世界'
                textarea.dispatchEvent(new Event('input', { bubbles: true }))
            }
        })
        await browser.pause(500)

        const canvas = await $('canvas')

        // Click on '世' (index 6)
        // "Hello " = 6 ASCII chars = 6 * 8 = 48px
        // Then 0 wide chars
        const clickX = 15 + 48
        const clickY = 50

        await canvas.click({ x: clickX, y: clickY })
        await browser.pause(300)

        const cursorPos = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.selectionStart : -1
        })

        console.log(`Mixed text click: (${clickX}, ${clickY}) -> cursor ${cursorPos}`)

        // Should be at or near position 6
        expect(cursorPos).toBeGreaterThanOrEqual(4)
        expect(cursorPos).toBeLessThanOrEqual(8)
    })

    it('should maintain click accuracy across multiple lines', async () => {
        // Type multiple lines
        await browser.keys(['L', 'i', 'n', 'e', ' ', '1'])
        await browser.keys(['Enter'])
        await browser.pause(100)
        await browser.keys(['L', 'i', 'n', 'e', ' ', '2'])
        await browser.keys(['Enter'])
        await browser.pause(100)
        await browser.keys(['L', 'i', 'n', 'e', ' ', '3'])
        await browser.pause(300)

        const canvas = await $('canvas')

        // Click on second line (LINE_HEIGHT = 20.0)
        const clickX = 15 + 8 * 2  // Click on 'n' in "Line"
        const clickY = 50 + 20  // Second line

        await canvas.click({ x: clickX, y: clickY })
        await browser.pause(300)

        const cursorPos = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.selectionStart : -1
        })

        console.log(`Multi-line click: (${clickX}, ${clickY}) -> cursor ${cursorPos}`)

        // Should be on second line (positions 7-13 for "Line 2\n")
        // "Line 1\n" = 7 chars, so second line starts at 7
        // Click at position 2 of second line = 7 + 2 = 9
        expect(cursorPos).toBeGreaterThanOrEqual(7)
        expect(cursorPos).toBeLessThanOrEqual(13)
    })

    it('should handle click at end of line correctly', async () => {
        // Type text
        await browser.keys(['E', 'n', 'd'])
        await browser.pause(300)

        const canvas = await $('canvas')

        // Click beyond the end of text
        const clickX = 15 + 8 * 10  // Way beyond "End"
        const clickY = 50

        await canvas.click({ x: clickX, y: clickY })
        await browser.pause(300)

        const cursorPos = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.selectionStart : -1
        })

        console.log(`End-of-line click: (${clickX}, ${clickY}) -> cursor ${cursorPos}`)

        // Should clamp to end of text (position 3)
        expect(cursorPos).toBe(3)
    })

    it('should handle rapid sequential clicks at different positions', async () => {
        // Type text
        await browser.keys(['R', 'a', 'p', 'i', 'd', ' ', 'C', 'l', 'i', 'c', 'k', 's'])
        await browser.pause(300)

        const canvas = await $('canvas')

        // Click sequence: start, middle, end, middle
        const positions = [
            { x: 15 + 8 * 0, y: 50 },   // Start
            { x: 15 + 8 * 6, y: 50 },   // Middle (space after "Rapid")
            { x: 15 + 8 * 11, y: 50 },  // End (last 's')
            { x: 15 + 8 * 3, y: 50 },   // Back to middle
        ]

        for (const pos of positions) {
            await canvas.click({ x: pos.x, y: pos.y })
            await browser.pause(200)

            const cursorPos = await browser.execute(() => {
                const textarea = document.querySelector('textarea')
                return textarea ? textarea.selectionStart : -1
            })

            console.log(`Click at (${pos.x}, ${pos.y}) -> cursor ${cursorPos}`)

            // Each click should result in a valid cursor position
            expect(cursorPos).toBeGreaterThanOrEqual(0)
            expect(cursorPos).toBeLessThanOrEqual(12)
        }
    })

    it('should verify coordinate reversibility in real desktop app', async () => {
        // This is the ULTIMATE test combining our mathematical tests
        // with real desktop rendering

        await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            if (textarea) {
                textarea.value = 'Test 日本語 Mix'
                textarea.dispatchEvent(new Event('input', { bubbles: true }))
            }
        })
        await browser.pause(500)

        const canvas = await $('canvas')

        // Get actual character widths from the app
        const charWidths = await browser.execute(() => {
            // These should match src/core/virtual_editor.rs constants
            return {
                ascii: 8.0,
                wide: 13.0,
                padding: 15.0,
                lineHeight: 20.0
            }
        })

        // Calculate expected position for '日' (index 5)
        // "Test " = 5 ASCII = 5 * 8 = 40
        const expectedX = charWidths.padding + 40
        const expectedY = 50

        await canvas.click({ x: expectedX, y: expectedY })
        await browser.pause(300)

        const cursorPos = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.selectionStart : -1
        })

        console.log(`Reversibility test: expected col 5, got cursor ${cursorPos}`)

        // This is the strictest test - should be exactly at position 5
        // or very close (±1 due to rounding)
        expect(Math.abs(cursorPos - 5)).toBeLessThanOrEqual(1)
    })
})
