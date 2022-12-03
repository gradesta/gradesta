/// <reference types="cypress" />

import kcfGraph from "../../kcfGraph";

describe("Unit tests", function () {

    before(() => {
        // check if the import worked correctly
        expect(kcfGraph.get_date_range, 'get_date_range').to.be.a('function')
    })

    context('kcfGraph.js', function () {
        expect(kcfGraph.get_date_range([])).to.eq((null, null))
    })
})
