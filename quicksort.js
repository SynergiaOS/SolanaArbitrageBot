/**
 * Comprehensive Quicksort Implementation in JavaScript
 * Multiple variants with different optimization strategies
 */

// Basic Quicksort Implementation
function quicksort(arr) {
    if (arr.length <= 1) {
        return arr;
    }
    
    const pivot = arr[Math.floor(arr.length / 2)];
    const left = [];
    const right = [];
    const equal = [];
    
    for (let element of arr) {
        if (element < pivot) {
            left.push(element);
        } else if (element > pivot) {
            right.push(element);
        } else {
            equal.push(element);
        }
    }
    
    return [...quicksort(left), ...equal, ...quicksort(right)];
}

// In-place Quicksort (more memory efficient)
function quicksortInPlace(arr, low = 0, high = arr.length - 1) {
    if (low < high) {
        const pivotIndex = partition(arr, low, high);
        quicksortInPlace(arr, low, pivotIndex - 1);
        quicksortInPlace(arr, pivotIndex + 1, high);
    }
    return arr;
}

function partition(arr, low, high) {
    const pivot = arr[high];
    let i = low - 1;
    
    for (let j = low; j < high; j++) {
        if (arr[j] <= pivot) {
            i++;
            [arr[i], arr[j]] = [arr[j], arr[i]]; // Swap elements
        }
    }
    
    [arr[i + 1], arr[high]] = [arr[high], arr[i + 1]]; // Place pivot
    return i + 1;
}

// Randomized Quicksort (better average performance)
function randomizedQuicksort(arr, low = 0, high = arr.length - 1) {
    if (low < high) {
        // Randomize pivot selection
        const randomIndex = Math.floor(Math.random() * (high - low + 1)) + low;
        [arr[randomIndex], arr[high]] = [arr[high], arr[randomIndex]];
        
        const pivotIndex = partition(arr, low, high);
        randomizedQuicksort(arr, low, pivotIndex - 1);
        randomizedQuicksort(arr, pivotIndex + 1, high);
    }
    return arr;
}

// Generic quicksort with custom comparator
function quicksortGeneric(arr, compareFn = (a, b) => a - b) {
    if (arr.length <= 1) return arr;
    
    const pivot = arr[Math.floor(arr.length / 2)];
    const left = arr.filter(x => compareFn(x, pivot) < 0);
    const equal = arr.filter(x => compareFn(x, pivot) === 0);
    const right = arr.filter(x => compareFn(x, pivot) > 0);
    
    return [...quicksortGeneric(left, compareFn), ...equal, ...quicksortGeneric(right, compareFn)];
}

// Hybrid Quicksort (switches to insertion sort for small arrays)
function hybridQuicksort(arr, threshold = 10) {
    if (arr.length <= threshold) {
        return insertionSort(arr);
    }
    
    const pivot = arr[Math.floor(arr.length / 2)];
    const left = arr.filter(x => x < pivot);
    const equal = arr.filter(x => x === pivot);
    const right = arr.filter(x => x > pivot);
    
    return [...hybridQuicksort(left, threshold), ...equal, ...hybridQuicksort(right, threshold)];
}

function insertionSort(arr) {
    const result = [...arr];
    for (let i = 1; i < result.length; i++) {
        let key = result[i];
        let j = i - 1;
        while (j >= 0 && result[j] > key) {
            result[j + 1] = result[j];
            j--;
        }
        result[j + 1] = key;
    }
    return result;
}

// Performance testing function
function performanceTest() {
    const sizes = [100, 1000, 10000];
    const algorithms = {
        'Basic Quicksort': quicksort,
        'In-place Quicksort': (arr) => quicksortInPlace([...arr]),
        'Randomized Quicksort': (arr) => randomizedQuicksort([...arr]),
        'Hybrid Quicksort': hybridQuicksort,
        'Native Array.sort()': (arr) => [...arr].sort((a, b) => a - b)
    };
    
    console.log('Performance Comparison:');
    console.log('='.repeat(50));
    
    sizes.forEach(size => {
        console.log(`\nArray size: ${size}`);
        const testArray = Array.from({length: size}, () => Math.floor(Math.random() * 1000));
        
        Object.entries(algorithms).forEach(([name, fn]) => {
            const start = performance.now();
            fn([...testArray]);
            const end = performance.now();
            console.log(`${name}: ${(end - start).toFixed(2)}ms`);
        });
    });
}

// Usage examples
function examples() {
    console.log('Quicksort Examples:');
    console.log('='.repeat(30));
    
    const numbers = [64, 34, 25, 12, 22, 11, 90];
    console.log("Original:", numbers);
    console.log("Basic Quicksort:", quicksort([...numbers]));
    console.log("In-place Quicksort:", quicksortInPlace([...numbers]));
    console.log("Randomized Quicksort:", randomizedQuicksort([...numbers]));
    console.log("Hybrid Quicksort:", hybridQuicksort([...numbers]));
    
    // Example with objects
    const people = [
        { name: "Alice", age: 30 },
        { name: "Bob", age: 25 },
        { name: "Charlie", age: 35 }
    ];
    
    console.log("\nSorting objects by age:");
    const sortedByAge = quicksortGeneric(people, (a, b) => a.age - b.age);
    console.log(sortedByAge);
    
    // Example with strings
    const words = ["banana", "apple", "cherry", "date"];
    console.log("\nSorting strings:");
    console.log(quicksortGeneric(words, (a, b) => a.localeCompare(b)));
}

// Export functions for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        quicksort,
        quicksortInPlace,
        randomizedQuicksort,
        quicksortGeneric,
        hybridQuicksort,
        performanceTest,
        examples
    };
}

// Run examples if this file is executed directly
if (typeof window === 'undefined' && require.main === module) {
    examples();
    performanceTest();
}

/**
 * Algorithm Complexity:
 * - Time Complexity:
 *   - Best/Average Case: O(n log n)
 *   - Worst Case: O(n²) - when pivot is always smallest/largest
 * - Space Complexity: O(log n) for in-place version, O(n) for basic version
 * 
 * Key Features:
 * - Multiple implementation strategies
 * - Randomized pivot selection for better average performance
 * - Generic version with custom comparison functions
 * - Hybrid approach combining quicksort with insertion sort
 * - Performance testing utilities
 * 
 * Best Practices:
 * - Use randomized pivot to avoid worst-case scenarios
 * - Consider hybrid approaches for better performance on small arrays
 * - For production use, JavaScript's Array.sort() is often sufficient
 */
