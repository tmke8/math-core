import {convert} from '../src/math_core.js';
import {assert} from 'chai';

describe('Convert Tests', function() {
    context('Simple command', function() {
        it('should convert simple command correctly', function() {
            const latex = 'x\\sum x';
            const prettyPrint = false;
            assert.equal(convert(latex, prettyPrint), '<math><mi>x</mi><mo>∑</mo><mi>x</mi></math>');
        });
    });
});
